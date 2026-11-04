# Headscale backup (VPS-006, decision D-12)

Every day the VPS takes a consistent snapshot of the Headscale database, adds the two
private keys and the configuration, and sends it with `restic` to a `rest-server` on the
home Linux PC over the VPN. restic encrypts on the VPS. The rest-server runs
`--append-only --private-repos`: a client can add files but never delete or overwrite
them. The VPS holds the repository key, so a compromised VPS can append crafted data
(snapshots, and index or pack files), not only real backups. `prune.sh` refuses anomalous
snapshot lists, so appended snapshots cannot evict real ones (see [Risks](#risks)); it
does not verify pack data before pruning (deferred, issue #1). Retention and pruning run
only on the home PC.

The full setup and the restore drill are in the VPN guide (`docs/setup/vpn.md`). This
file lists what goes where. The hub reaches `10.10.0.0/24` only if the VPS runs
`tailscale up --accept-routes` (also in the guide).

## File map

systemd needs the `.service` suffix; the repository files end in `.unit` because the
name check (CQ-030) refuses the word "service" in file names. Copy them to the
installed names below.

| Repository file | Machine | Installed as | Mode |
| --- | --- | --- | --- |
| `backup.sh` | VPS | `/usr/local/sbin/headscale-backup` | `0700 root` |
| `headscale-backup.unit` | VPS | `/etc/systemd/system/headscale-backup.service` | `0644` |
| `headscale-backup.timer` | VPS | `/etc/systemd/system/headscale-backup.timer` | `0644` |
| `rest-server.unit` | home PC | `/etc/systemd/system/rest-server.service` | `0644` |
| `rest-server-allow.conf` | home PC | `/etc/systemd/system/rest-server.service.d/allow.conf`, placeholder replaced | `0644` |
| `prune.sh` | home PC | `/usr/local/bin/restic-prune` | `0755` |
| `restic-prune.unit` | home PC | `/etc/systemd/system/restic-prune.service` | `0644` |
| `restic-prune.timer` | home PC | `/etc/systemd/system/restic-prune.timer` | `0644` |

Files the operator creates (secrets: never commit them, never print them):

| File | Machine | Content | Mode |
| --- | --- | --- | --- |
| `/etc/anima/backup/restic-repository` | VPS | `rest:http://headscale-vps:<rest-server password>@<home PC LAN IP>:8000/headscale-vps/headscale` | `0600 root` |
| `/etc/anima/backup/restic-password` | VPS | restic encryption password (also kept offline) | `0600 root` |
| `/etc/restic/htpasswd` | home PC | `htpasswd -B` entry for user `headscale-vps` | `0640 root:restic` |
| `/etc/restic/headscale-password` | home PC | the same restic encryption password | `0600 root` |
| `/etc/restic/rest-server.env` | home PC | `HOME_PC_LAN_IP=10.10.0.X` (not a secret) | `0644` |

Generate the rest-server password in your password manager, letters and digits only and at
least 32 characters: it sits inside a URL, and letters and digits need no URL encoding.
Type it at the prompts in the guide (step 9); never print it with a command.

Packages: on the VPS (AlmaLinux 10), `sqlite` (AppStream, the `sqlite3` command) and
`restic` (EPEL 10); both install to `/usr/bin`. Under SELinux the backup unit runs as
`unconfined_service_t` and reads Headscale's `container_file_t` files; the systemd sandbox
confines it. On the home PC, `restic`, `rest-server` and `jq`, with a system user `restic` that owns `/srv/restic` (`0700`). Put `/srv/restic` on its
own filesystem or under a quota, so a flood of uploads cannot fill the home PC's disk
before `--max-size` (1 GiB) stops it.

## What a snapshot holds

From `/opt/anima/headscale`: `lib/db.sqlite` (taken with `VACUUM INTO` and checked with
`PRAGMA integrity_check`), `lib/noise_private.key`, `lib/derp_server_private.key`,
`config/config.yaml`, `config/policy.hujson`. Host `anima-vps`, tag `headscale`. The ACME
cache is not saved; Headscale requests a new certificate.

## Run and check

```sh
# VPS
systemctl daemon-reload
systemctl enable --now headscale-backup.timer
systemctl start headscale-backup.service        # first run by hand
journalctl -u headscale-backup.service -n 50

# home PC
systemctl daemon-reload
systemctl enable --now rest-server.service restic-prune.timer
```

First real run on the VPS, check:

1. The database copy works under `ReadOnlyPaths`. `sqlite3 -readonly` opens the live WAL
   database read-only. If the journal shows `unable to open database file (14)` while
   Headscale is stopped, start Headscale once (it creates `db.sqlite-wal` and
   `db.sqlite-shm`) and run the backup again. If the error persists, add
   `ReadWritePaths=-/opt/anima/headscale/lib/db.sqlite-shm` to the unit (the `-shm` file
   only, never the whole folder; the leading `-` keeps the unit from failing with
   226/NAMESPACE when the file is missing). This lets the backup process write the live
   WAL index, so add it only if the read-only open fails.
2. `ls -ln /opt/anima/headscale/lib` shows owner `0` for every file. The unit has no
   capabilities; if Headscale's files belong to another user, add
   `CapabilityBoundingSet=CAP_DAC_READ_SEARCH` to the unit.
3. On the home PC, `journalctl -u rest-server.service` shows the VPS's request coming
   from the address in `allow.conf`.

## Restore

Both cases below take only `db.sqlite` and the two private keys from a snapshot. They
render `config.yaml` again with `render.sh` and install `policy.hujson` from the
repository, never from the snapshot: a snapshot written by a taken-over VPS can hold a
widened policy or a changed config.

**After a suspected compromise, restore onto a freshly reinstalled VPS** (Hostinger
reinstall, or a new VPS), never onto the same system: the "same VPS" steps below are for
a lost or corrupted database only. The restored `noise_private.key` is known to the
attacker, and rotating it means registering every node again, so it is kept unless the
programme decides otherwise (WP-03 D-12).

### 1. Choose the snapshot by ID (home PC)

The VPS sets each snapshot's time itself, so `latest` and the `Time` column can point to a
snapshot a taken-over VPS wrote, even a back-dated one. The home PC records when each
snapshot file arrived: each file in `snapshots/` is named by the full snapshot ID, and its
time comes from the home PC's clock. Run as root:

```sh
# home PC
restic -r /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password \
  --no-cache --no-lock snapshots --host anima-vps --tag headscale
ls -l --time-style=+%Y-%m-%dT%H:%M:%S%z /srv/restic/headscale-vps/headscale/snapshots/
```

Pick the `<SNAPSHOT_ID>` whose file arrived before the suspected compromise; if you do not
know when it began, take one from before the earliest sign. **After an incident never use
`latest`.** The quarterly drill in the guide restores `latest` into a scratch folder only
to prove that the backup can be read and decrypted; it puts nothing into service, so a
poisoned latest snapshot does no harm there.

### 2. Restore the three files (home PC)

Restore on the home PC, not on the VPS: the VPS may be the compromised machine. Without a
cache or a lock, nothing is written into the `restic`-owned repository. A taken-over VPS
can write a crafted snapshot, so restore only the three files, and refuse links: `install`
follows a link, and a link named `db.sqlite` could copy any file of the home PC. Run as the
admin user of the home PC (`<HOME_USER>`):

```sh
# home PC
sudo restic -r /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password \
  --no-cache --no-lock restore <SNAPSHOT_ID> --target /root/headscale-restore \
  --include '/tmp/headscale-backup.*/db.sqlite' --include '/tmp/headscale-backup.*/noise_private.key' \
  --include '/tmp/headscale-backup.*/derp_server_private.key'
install -d -m 0700 ~/headscale-take
sudo sh -euc '
  restored=/root/headscale-restore
  [ -z "$(find "$restored" -type l)" ] || { echo "STOP: the snapshot holds links"; exit 1; }
  src=$(find "$restored" -type f -name db.sqlite -printf "%h\n")
  [ "$(printf "%s\n" "$src" | grep -c .)" = 1 ] || { echo "STOP: not exactly one db.sqlite"; exit 1; }
  for f in "$src/db.sqlite" "$src/noise_private.key" "$src/derp_server_private.key"; do
    [ -f "$f" ] && [ ! -L "$f" ] || { echo "STOP: $f is missing or not a plain file"; exit 1; }
  done
  sqlite3 "$src/db.sqlite" "PRAGMA integrity_check"         # must print ok
  install -m 0600 -o "$1" "$src/db.sqlite" "$src/noise_private.key" "$src/derp_server_private.key" "$2/headscale-take/"
' sh "$USER" "$HOME"
sudo rm -rf /root/headscale-restore                           # it holds private keys
```

If it prints `STOP`, do not use that snapshot: take an older one and send the output to
the programme.

Copy them to the VPS through your Mac (the policy lets the Mac reach the home subnet and
the hub's SSH; it lets the home PC reach neither). `scp -3` passes the files through the
Mac without storing them there:

```sh
# operator's Mac
scp -3 -r <HOME_USER>@<HOME_PC_LAN_IP>:headscale-take <ADMIN_USER>@<VPS_ADDRESS>:headscale-take
ssh <HOME_USER>@<HOME_PC_LAN_IP> rm -rf headscale-take
```

`<VPS_ADDRESS>` is the hub's VPN address for the same VPS, or the public address of a new
VPS before its firewall (guide step 4.6).

### 3. Put them in place (VPS)

A new VPS first gets guide steps 2.0 to 2.6 (inventory, admin user, packages, folders,
`render.sh` with the **new** public IPv4, compose and policy from the repository, SELinux
labels, digest check), but not step 2.7: starting Headscale there would create a fresh
database and keys. On the same VPS, copy a fresh repository folder from your Mac to
`~/anima-headscale` first (guide step 2.4). Then, as `<ADMIN_USER>`:

```sh
# VPS
cd /opt/anima/headscale
sudo docker compose stop headscale                    # same VPS only
sudo cp -a lib /root/headscale-lib.before-restore     # same VPS only: the undo; it holds private keys
sudo rm -f lib/db.sqlite-wal lib/db.sqlite-shm        # a stale WAL would corrupt the restored db
sudo install -m 0600 ~/headscale-take/db.sqlite lib/db.sqlite
sudo install -m 0600 ~/headscale-take/noise_private.key ~/headscale-take/derp_server_private.key lib/
rm -rf ~/headscale-take
cd ~/anima-headscale
sudo env VPS_PUBLIC_IPV4=<VPS_PUBLIC_IPV4> ACME_EMAIL=<YOUR_EMAIL> \
  bash ./config/render.sh ./config/config.yaml /opt/anima/headscale/config/config.yaml
sudo install -m 0644 ./config/policy.hujson /opt/anima/headscale/config/policy.hujson
sudo restorecon -R /opt/anima/headscale
sudo ls -l /opt/anima/headscale/config /opt/anima/headscale/lib    # config.yaml and lib files -rw------- root root
sudo docker compose -f /opt/anima/headscale/compose.yaml up -d
```

Undo on the same VPS, before step 4: `compose stop`, `sudo rm -rf lib`,
`sudo mv /root/headscale-lib.before-restore lib`, `sudo restorecon -R lib`, `compose start`.
Once the restore is accepted, `sudo rm -rf /root/headscale-lib.before-restore`.

### 4. Replace the hub node, then review everything (VPS)

The old `hub` node's key was on the VPS. Delete it and join again with a new tagged
pre-auth key (guide steps 3.1 to 3.3). On the same VPS, `tailscale logout` cuts an SSH
session over the VPN, so do this part at the Hostinger browser console:

```sh
# VPS (Hostinger browser console on the same VPS; SSH on a new VPS)
hsctl() { sudo docker compose -f /opt/anima/headscale/compose.yaml exec -T headscale headscale "$@"; }
sudo tailscale logout                                 # same VPS only
hsctl nodes list                                      # note the ID of the old hub
hsctl nodes delete --identifier <OLD_HUB_NODE_ID>
# then guide steps 3.1 to 3.3: a new tag:hub pre-auth key, tailscale up, tailscale ip -4
```

Then review every node, pre-auth key and API key against what you expect (guide step 11,
Detection): `hsctl nodes list` (each node, its tags and expiry), `hsctl nodes list-routes`
(only routes you approved), `hsctl preauthkeys list` (expire every unused key), and
`hsctl apikeys list` (empty). Delete or expire anything you did not create.

### 5. Follow the addresses (home PC, Cloudflare)

- If `tailscale ip -4` on the VPS changed, update the hub address in the home PC's
  `allow.conf` and the `hub` DNS record (guide "Hub VPN address changes").
- On a new VPS, update the `hs` A record to its public IPv4 (`deploy/dns/README.md`; delete
  the old record, then apply the `hs` row again) before Headscale asks for its
  certificate, and set the Hostinger panel firewall for it (guide step 1).

## Risks

- **Snapshot flooding.** The VPS sets its own snapshot times. Retention is
  `--keep-within 1y` only: real snapshots are kept for one year whatever the VPS
  appends, and anything older than a year (counted from the newest snapshot) is pruned.
  Because keep-within counts from the newest snapshot, `prune.sh` refuses to prune when a
  snapshot is dated more than an hour in the future. It also refuses when a UTC day has
  more than two snapshots, which bounds growth (about 365 snapshots of a database under
  1 MB, well inside `--max-size`). After a refusal, inspect
  `restic snapshots --host anima-vps --tag headscale` and forget the bad snapshot IDs by
  hand. One extra run by hand beside the timer is fine; two or more extra runs on the same
  UTC day trigger a refusal.
- **The home PC holds the restic password**, because pruning needs it. Anyone with root on
  the home PC can decrypt the backups, including the Headscale private keys.
- **rest-server and prune share the `restic` user.** rest-server writes the repository
  files, so its user owns them: a compromised rest-server can delete the repository
  whichever user runs prune. Append-only protects against a compromised client (the
  VPS), not a compromised server. A separate prune user would not fix that. It would also
  have to share write access to the repository folders, and a prune run as root would
  leave files rest-server cannot read. Prune therefore runs as `restic`. The cost: while
  prune runs, a process running as `restic` could read the password credential. The
  defences are around rest-server: only the hub's VPN address may connect, it listens on
  the LAN address only, and it runs sandboxed with no capabilities.
- **Stale backups are not alerted yet** (deferred, issue #1). Check
  `systemctl list-timers headscale-backup.timer` and the journal.
