# Desktop releases

Spec: Section 3.9 (UPD-001 to UPD-012) and Section 3.10 (BLD-001 to BLD-082), DNS-007, R-107 and
R-108. A release is a feature of Anima: an operator computer on the VPN builds it, uploads it to the
R2 bucket `anima-releases`, and the hub verifies it and writes the feed. There is no release
workflow and no GitHub release.

## How an installed app updates

1. The app reads `https://releases.anima.ing/{{target}}-{{arch}}/latest.json` at start and every 4
   hours (UPD-001). Tauri replaces `{{target}}` with `linux` or `darwin` and `{{arch}}` with
   `x86_64` or `aarch64`, so each target has its own feed (BLD-067). The URL is
   `plugins.updater.endpoints` in `apps/desktop/src-tauri/tauri.conf.json`; the `main` window has
   only `updater:allow-check`, `updater:allow-download-and-install` and `process:allow-restart`
   (`capabilities/desktop.json`).
2. A feed whose version differs from the running one is offered. With `allowDowngrades: true` an
   older feed is offered too, which is how a rollback installs (BLD-069).
3. The plugin downloads the file and verifies its minisign signature with `plugins.updater.pubkey`
   before it installs anything (UPD-002). With `requireSignedVersion` the signature must also name
   the announced version in its trusted comment.
4. Settings > Updates says what happened, with no dialog (UPD-006):
   - **Up to date.**
   - **No update is published for this computer yet.** The feed answered 404, or lists no platform
     for this target.
   - **Updates for the Mac are not published yet.** The same, on a Mac. The hub writes no macOS feed
     until the build has Developer ID signing and notarization (BLD-068, OD-13); install a new version
     from its disk image.
   - **An update was refused because its signature did not verify.** Nothing is installed and the
     app keeps running; the process log has "Update signature refused."
   - **The last update attempt failed.** The network, or the disk. The next check runs at the usual
     time.

## Once, by the operator

Agents never do these steps (UPD-012, VPS-005, DNS-008). Steps marked 🔒 touch a secret: never paste
the value into chat, a commit or a log.

**a. 🔒 Make the update key pair** (file `~/.tauri/anima-updater.key`, public half
`~/.tauri/anima-updater.key.pub`). Use the repository's own CLI. Do not run `tauri` through bunx or
npx: that name resolves to an old unrelated npm package.

```sh
bun run --cwd apps/desktop tauri signer generate -w ~/.tauri/anima-updater.key
chmod 600 ~/.tauri/anima-updater.key
```

Back up the private key and its password offline: attach the key file to an entry in your password
manager and store the password in the same entry, then keep an encrypted copy on a USB stick in a
safe place. If you lose them, every installed app must be installed again by hand. Then import the
private key into the key store of each builder from Settings > Builds (BLD-026).

**b. Public key into the app and the hub.** Put the text of `~/.tauri/anima-updater.key.pub` in
`tauri.conf.json` as `plugins.updater.pubkey` (set; key ID 5943DAEA7084E677),
commit it, and set the same text as `ANIMA_UPDATER_PUBLIC_KEY` on the hub (BLD-064). The public key
is not a secret. A builder whose app still has the placeholder claims no job.

**c. The R2 custom domain (DNS-007).** In Cloudflare, make `releases.anima.ing` the custom domain of
the bucket `anima-releases`, and set the hub's R2 variables (`deploy/README.md`). Check:

```sh
curl -sI https://releases.anima.ing/linux-x86_64/latest.json
```

It answers 404 until the first release and 200 after it. There is no `updates.anima.ing` record and
no redirect rule; delete them in the zone if an older setup made them.

**d. 🔒 Optional: Developer ID signing and notarization.** With a Developer ID the macOS builder
signs and notarizes, and the hub can write the Mac feeds. Keys stay in the builder's key store.

**e. Keep the release path hard to attack.** Turn on two-factor authentication on Cloudflare. With
`allowDowngrades: true`, anyone who can write the bucket can send apps back to any older signed
release; `requireSignedVersion` stops mislabelled versions, not rollbacks. The protection is the
Cloudflare account and the hub being the only writer of `latest.json`.

## Roll back

The hub refuses a feed older than the published one (BLD-069), so a rollback is a manual act. Write
the older release's `latest.json` to `<target>/latest.json` in the bucket by hand (the old release's
files stay in their own `<target>/<version>/` folders). The apps run with `allowDowngrades`, so at
their next check they install that version. Do not delete the bad release's files until the rollback
is in place.

## Proof that the updater picks the newer build and refuses a bad one

**Automated (a gate).** `apps/desktop/src-tauri/tests/updater/` runs the real `tauri-plugin-updater`,
configured from the shipped `plugins.updater` block, against a feed on 127.0.0.1. Each test makes a
throwaway minisign key pair in memory and puts its public half in place of the placeholder; no key
is stored or read. It proves:

- the app asks for `/<target>-<arch>/latest.json` (the shipped URL is `releases.anima.ing`);
- the newer build is offered and its downloaded bytes verify; the same version is not offered again;
- an older signed build installs as a rollback (`allowDowngrades`);
- a signature by another key, bytes changed after signing, a signature made for another version and a
  signature that names no version (`requireSignedVersion`) are each refused;
- a 404 feed and a feed with no entry for this target install nothing.

Run it alone with `cargo test -p anima-desktop --test updater`.

**Manual, two AppImages (needs a Linux computer and a release build).** The automated proof does
not run the installer or the restart, so do this once with real files:

1. Make a throwaway key: `bun run --cwd apps/desktop tauri signer generate -w /tmp/proof.key` (empty
   password). Delete `/tmp/proof.key*` when you finish. Never use the operator's key.
2. Build version A and version B (B higher) with a release `tauri build`, a `--config` that sets
   `plugins.updater.pubkey` to the throwaway public key and `plugins.updater.endpoints` to
   `["http://127.0.0.1:8099/{{target}}-{{arch}}/latest.json"]` with
   `dangerousInsecureTransportProtocol: true`, and `TAURI_SIGNING_PRIVATE_KEY` set to the throwaway
   key. Sign B with `tauri signer sign --app-version <B>`.
3. Serve a folder with `linux-x86_64/latest.json` (version B, `platforms."linux-x86_64"` with B's
   `.sig` text and a URL on 127.0.0.1:8099) and B's AppImage, for example `python3 -m http.server`.
4. Run A. Settings shows "Anima B is installed — restart to update"; "Restart to update" runs B.
5. Repeat with a `.sig` from a different key. Settings says the update was refused, the log has
   "Update signature refused.", and A keeps running.
