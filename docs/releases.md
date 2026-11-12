# Desktop releases

Spec: Sections 3.9 (UPD-001 to UPD-012) and 3.10 (Builds), DNS-007.

A release starts only from the Builds view (BLD-001, UPD-007). The operator's builders build and sign
each target, upload the files to the R2 bucket `anima-releases`, and the hub checks them and writes
`https://releases.anima.ing/<target>-<arch>/latest.json` (BLD-064 to BLD-067). Installed apps read
that feed. The hub side, its R2 credentials and the bucket setup are in
[deploy/README.md](../deploy/README.md).

There is no GitHub release workflow: `.github/workflows/release.yml` was removed by R-114. No
signing key is stored on GitHub.

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
3. On each builder, open Settings, Updates, and choose "Import updater key" (UPD-012). The app
   keeps the key sealed; before a build, "Unlock updater key" asks for its password. The key is
   never in GitHub or in a file the agents read.

A plain local `tauri build` without the key still builds the app, but it makes no signed updater
files. Updater files come only from the Builds flow.
