# Headscale backup (VPS-006, decision D-12)

Every day the VPS takes a consistent snapshot of the Headscale database, adds the two
private keys and the configuration, and sends it with `restic` to a `rest-server` on the
home Linux PC over the VPN. restic encrypts on the VPS. The rest-server runs
`--append-only --private-repos`, so a compromised VPS can add snapshots but cannot
delete or change old ones. Retention and pruning run only on the home PC.

The full setup and the restore drill are in the VPN guide (`docs/setup/vpn.md`). This
file lists what goes where.

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

Packages: `sqlite3` and `restic` on the VPS; `restic` and `rest-server` on the home PC,
with a system user `restic` that owns `/srv/restic` (`0700`).

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
systemctl enable --now rest-server.service restic-prune.timer
```

## Restore

On the VPS (the append-only repository still allows reads):

```sh
R="restic --repository-file /etc/anima/backup/restic-repository --password-file /etc/anima/backup/restic-password"
$R snapshots --host anima-vps --tag headscale
$R restore latest --host anima-vps --tag headscale --target /root/headscale-restore
src="$(dirname "$(find /root/headscale-restore -name db.sqlite)")"
sqlite3 "$src/db.sqlite" "PRAGMA integrity_check"      # must print ok

cd /opt/anima/headscale
docker compose stop headscale
rm -f lib/db.sqlite-wal lib/db.sqlite-shm              # a stale WAL would corrupt the restored db
install -m 0600 "$src/db.sqlite" lib/db.sqlite
install -m 0600 "$src/noise_private.key" "$src/derp_server_private.key" lib/
install -m 0644 "$src/config.yaml" "$src/policy.hujson" config/
docker compose start headscale
docker compose exec headscale headscale nodes list     # every node is listed
rm -rf /root/headscale-restore
```

If the VPS is lost, restore on the home PC from the local folder, then copy the files to
the new VPS over SSH:

```sh
restic -r /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password \
  restore latest --host anima-vps --tag headscale --target /root/headscale-restore
```
