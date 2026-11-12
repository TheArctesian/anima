# Desktop releases

Spec: Sections 3.9 (UPD-001 to UPD-012) and 3.10 (Builds), DNS-007.

A release starts only from the Builds view (BLD-001, UPD-007). The operator's builders build and sign
each target, upload the files to the R2 bucket `anima-releases`, and the hub checks them and writes
`https://releases.anima.ing/<target>-<arch>/latest.json` (BLD-064 to BLD-067). Installed apps read
that feed. The hub side, its R2 credentials and the bucket setup are in
[deploy/README.md](../deploy/README.md).

There is no GitHub release workflow: `.github/workflows/release.yml` was removed by R-114. The
operator keeps a `release` environment with Apple and updater secrets on GitHub (R-116), but nothing
in the repository reads them.

## Once, by the operator

Agents never do these steps (UPD-012). Never paste a key or password into chat, a commit or a log.

1. Make the updater key pair with the repository's own CLI (not through bunx or npx, where the name
   `tauri` is an old unrelated package):

   ```sh
   bun run --cwd apps/desktop tauri signer generate -w ~/.tauri/anima-updater.key
   chmod 600 ~/.tauri/anima-updater.key
   ```

   Back up the key and its password offline. If they are lost, every installed app must be
   installed again by hand.
2. Put the text of `~/.tauri/anima-updater.key.pub` in `apps/desktop/src-tauri/tauri.conf.json` as
   `plugins.updater.pubkey`, and commit it. The public key is not a secret.
3. On each macOS builder, install the Developer ID Application certificate that
   `bundle.macOS.signingIdentity` names into the login keychain (BLD-083), and store the
   notarization credentials once in the keychain profile `anima-notary` (BLD-084). Make an
   app-specific password at account.apple.com; the command asks for it:

   ```sh
   xcrun notarytool store-credentials anima-notary --apple-id <your Apple account> --team-id 7GDLG48PV6
   ```
4. On each builder, open Settings, Updates, and choose "Import updater key" (UPD-012). The app
   keeps the key sealed; before a build, "Unlock updater key" asks for its password. The key is
   never in GitHub or in a file the agents read.

A plain local `tauri build` without the key still builds the app, but it makes no signed updater
files. Updater files come only from the Builds flow.

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
   - **Updates for the Mac are not published yet.** The same, on a Mac. The hub writes no macOS feed
     until the build has Developer ID signing and notarization (BLD-068, OD-13).
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
