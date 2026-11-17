# Desktop releases

## Release record

This table is the only record of what is released (PLN-016). A different document that needs the state of a release points to this table; it does not copy it. Change the table in the commit that records a release or a check.

| Target | Feed | Newest version | State | Evidence |
| --- | --- | --- | --- | --- |
| macOS (`darwin-aarch64`, `darwin-x86_64`; one universal bundle) | `https://releases.anima.ing/darwin-aarch64/latest.json`, `https://releases.anima.ing/darwin-x86_64/latest.json` | 0.2.1 (2026-11-17, commit `b361eb5f`). 0.2.0 was the first release (2026-11-17, commit `43291afc`) | Published, signed with the Developer ID and notarized | [s12-packaging-tasks.md](../plans/one-click/s12-packaging-tasks.md) Section 6.4. Both feeds answered 200 with 0.2.1 on 2026-11-17 (`curl`) |
| Linux (`linux-x86_64`) | `https://releases.anima.ing/linux-x86_64/latest.json` | None | Not published | The feed answered 404 on 2026-11-17 (`curl`) |

The update of an installed application from the feed (OD-16 to OD-18 of the design specification): [hosted-mode.md](../proposals/hosted-mode.md), Section 3, says that the operator saw the update work on a second Mac with 0.2.0 on 2026-11-16. No entry of [need-human-testing.md](../need-human-testing.md) holds that result yet: its row REL-1 ("A real release") is open, and the operator sets it. Linux has no release, so nothing is verified there.

The releases 0.2.0 and 0.2.1 carry the program under its old name, `anima-agent`, and without its version report: the commits of R-122 and R-126 are not in them.

Spec: Sections 3.9 (UPD-001 to UPD-012) and 3.10 (Builds), DNS-007.

A release starts only from the Builds view (BLD-001, UPD-007). The operator's builders build each
target and upload the unsigned files to the R2 bucket `anima-releases`. The hub checks them, signs
the updater file with the updater key it holds, writes `<file>.sig` beside it and then
`https://releases.anima.ing/<target>-<arch>/latest.json` (BLD-064 to BLD-067, R-117). Installed apps read
that feed. The hub side, its R2 credentials and the bucket setup are in
[deploy/README.md](../../deploy/README.md).

There is no GitHub release workflow: `.github/workflows/release.yml` was removed by R-114. No
builder holds the updater signing key; it is a secret file of the hub (R-117). A
compromised hub can therefore sign updates that installed apps trust; the operator accepted this
in R-117.

The operator also keeps a `release` environment on GitHub with the Apple secrets and a copy of
the updater key, `TAURI_SIGNING_PRIVATE_KEY` (R-116). Nothing in the repository reads them.

## Once, by the operator

Agents never do these steps (UPD-012). Never paste a key or password into chat, a commit or a log.

1. Make the updater key pair with the repository's own CLI (not through bunx or npx, where the name
   `tauri` is an old unrelated package):

   ```sh
   bun run --cwd apps/desktop tauri signer generate -w ~/.config/anima/updater.key
   chmod 600 ~/.config/anima/updater.key
   ```

   Back up the key and its password offline. If they are lost, every installed app must be
   installed again by hand.
2. Put the text of `~/.config/anima/updater.key.pub` in `apps/desktop/src-tauri/tauri.conf.json` as
   `plugins.updater.pubkey`, and commit it. The public key is not a secret.
3. On each macOS builder, install the Developer ID Application certificate that
   `bundle.macOS.signingIdentity` names into the login keychain (BLD-083), and store the
   notarization credentials once in the keychain profile `anima-notary` (BLD-084). Make an
   app-specific password at account.apple.com; the command asks for it:

   ```sh
   xcrun notarytool store-credentials anima-notary --apple-id <your Apple account> --team-id 7GDLG48PV6
   ```
4. Put the key on the VPS: copy `~/.config/anima/updater.key` there yourself and install it, with
   its password, as the hub's secret files `secrets/updater/updater_key` and `secrets/updater/updater_key_password`
   ("Release publishing" in [deploy/README.md](../../deploy/README.md), UPD-012). Then delete the copy
   in your home folder on the VPS. The key is never on a builder or in a file the agents read. Check the files as root, with no `.env`: `sudo sh bin/preflight secrets` (only root and
   the hub's group can search `secrets/updater/`).
5. Deploy the hub, then check that its start log shows publishing true on its `release bucket`
   line and holds no "publishing is off" warning
   (`sudo docker compose logs hub | grep -i publishing`). A key file the hub cannot read leaves
   the hub running with publishing off.

6. On each computer that was a builder before R-117, the old sealed key may still be in the key
   store. The app deletes it at start; to delete it by hand:

   ```sh
   # Linux (Secret Service)
   secret-tool clear service ing.anima.desktop account updater-key
   secret-tool clear service ing.anima.desktop account updater-key-public
   # macOS (Keychain)
   security delete-generic-password -s ing.anima.desktop -a updater-key
   security delete-generic-password -s ing.anima.desktop -a updater-key-public
   ```

A builder holds no updater key and signs nothing. A plain local `tauri build` makes no signed
updater files; they come only from the hub in the Builds flow. If the hub cannot open the key, or
it is not the key of `ANIMA_UPDATER_PUBLIC_KEY`, the hub starts with publishing off and its log
says why.

## What a release carries

The bundle step of a builder builds, from the job's commit, every program that "Create a hub
on this computer" needs, and puts them into the application (plan item S12,
[s12-packaging-tasks.md](../plans/one-click/s12-packaging-tasks.md)):

| File | macOS | Linux AppImage |
| --- | --- | --- |
| `anima`, `anima-local-hub`, `anima-hub`, `anima-database`, `anima-gateway`, `anima-link` | `Anima.ing.app/Contents/MacOS/` | `usr/bin/` |
| The PostgreSQL tree | `Anima.ing.app/Contents/Resources/database/` | `usr/lib/Anima.ing/database/` |

- The Rust programs come from one `cargo build`. `anima-link` comes from `link/build.sh`, the
  tree from `apps/desktop/src-tauri/database-tree/build.sh`. The builder runs the two scripts
  with `/bin/sh` from its clone. The tree is built from the pinned source at each release; the
  source archive is downloaded from `ftp.postgresql.org` and checked against
  `database-tree/postgresql.pin`.
- A builder needs Go (the gates need it also). A Linux builder also needs `patchelf`, `bison`,
  `flex`, `readelf`, `make` and the zlib headers; the tool check in Settings names them when
  one is missing.
- The order is: the tree, then `anima-link` (after `go mod verify`, with a fixed Go
  environment), then the Rust programs, then Tauri. The two scripts, the pin, the Go module
  files and `tauri.conf.json` must be the files of the commit directly before their use.
- macOS signing: the builder signs with the identity that the builder application itself was
  built with, and the commit must name the same identity in `bundle.macOS.signingIdentity`.
  So a new identity needs a new builder application first. The recipe signs each file of the
  tree with the hardened runtime and a secure timestamp. Then Tauri signs each program in
  `Contents/MacOS`, then the application. No program has an entitlement.
- Before the builder sends the DMG to the notary service, it checks **each Mach-O file** of the
  application: exactly the two architectures, a Developer ID Application signature of the team that the
  identity names, the hardened runtime, a secure timestamp, no entitlement (UPD-011). A file
  that fails stops the job.
- The updater file holds all of this, so an update replaces the programs in the application.
  The hub runs a copy of them; the new application replaces that copy at its first start
  ([setup/local-hub.md](setup/local-hub.md), Section 3).

### A trial bundle (never a release)

`apps/desktop/src-tauri/trial-bundle.sh [version]` makes the same application on the computer
that runs it, with an ad hoc signature on macOS and no notary step. Use it to try the hub from
a built application. It uses no signing identity and uploads nothing. Gatekeeper refuses such
an application on another Mac.

## How an installed app updates

1. The app reads `https://releases.anima.ing/{{target}}-{{arch}}/latest.json` at start and every 4
   hours (UPD-001). Tauri replaces `{{target}}` with `linux` or `darwin` and `{{arch}}` with
   `x86_64` or `aarch64`, so each target has its own feed (BLD-067). The URL is
   `plugins.updater.endpoints` in `apps/desktop/src-tauri/tauri.conf.json`; the `main` window has
   only `updater:allow-check`, `updater:allow-download-and-install` and `process:allow-restart`
   (`capabilities/desktop.json`).
2. A feed whose version differs from the running one is offered. With `allowDowngrades: true` an
   older feed is offered too, which is how a rollback installs. The hub refuses to write an older
   feed (BLD-069), so a rollback is a manual write of an older `latest.json` to the bucket.
3. The plugin downloads the file and verifies its minisign signature with `plugins.updater.pubkey`
   before it installs anything (UPD-002). With `requireSignedVersion` the signature must also name
   the announced version in its trusted comment.
4. Settings > Updates says what happened, with no dialog (UPD-006):
   - **Up to date.**
   - **No update is published for this computer yet.** The feed answered 404, or lists no platform
     for this target.
   - **An update was refused because its signature did not verify.** Nothing is installed and the
     app keeps running; the process log has "Update signature refused."
   - **The last update attempt failed.** The network, or the disk. The next check runs at the usual
     time.

The operator's public key (ID 5943DAEA7084E677) is set as `plugins.updater.pubkey`; the hub reads
the same text as `ANIMA_UPDATER_PUBLIC_KEY` (BLD-064).

## Proof that the updater picks the newer build and refuses a bad one

**Automated (a gate).** `apps/desktop/src-tauri/tests/updater/` runs the real `tauri-plugin-updater`,
configured from the shipped `plugins.updater` block, against a feed on 127.0.0.1. Each test makes a
throwaway minisign key pair in memory and puts its public half in place of the shipped key; no key is
stored or read. It proves:

- the app asks for `/<target>-<arch>/latest.json` (the shipped URL is `releases.anima.ing`);
- the newer build is offered and its downloaded bytes verify; the same version is not offered again;
- an older signed build installs as a rollback (`allowDowngrades`);
- a signature by another key, bytes changed after signing, a signature made for another version and a
  signature that names no version (`requireSignedVersion`) are each refused;
- a 404 feed and a feed with no entry for this target install nothing.

Run it alone with `cargo test -p anima-desktop --test updater`.

**Manual, two AppImages (a Linux computer and a release build).** The automated proof does not run
the installer or the restart:

1. Make a throwaway key: `bun run --cwd apps/desktop tauri signer generate -w /tmp/proof.key` (empty
   password). Delete `/tmp/proof.key*` when you finish. Never use the operator's key.
2. Build version A and version B (B higher) with a release `tauri build` and a `--config` that sets
   `plugins.updater.pubkey` to the throwaway public key and `plugins.updater.endpoints` to
   `["http://127.0.0.1:8099/{{target}}-{{arch}}/latest.json"]` with
   `dangerousInsecureTransportProtocol: true`. Sign B with `tauri signer sign --app-version <B>`.
3. Serve a folder with `linux-x86_64/latest.json` (version B, `platforms."linux-x86_64"` with B's
   `.sig` text and a URL on 127.0.0.1:8099) and B's AppImage, for example `python3 -m http.server`.
4. Run A. Settings shows "Anima B is installed — restart to update"; "Restart to update" runs B.
5. Repeat with a `.sig` from a different key. Settings says the update was refused, the log has
   "Update signature refused.", and A keeps running.
