# Desktop releases

Spec: Section 3.9 (UPD-001 to UPD-012) and DNS-007. The workflow is `.github/workflows/release.yml`.

## How a release happens

1. A push to `main` (or "Run workflow" by hand) starts the workflow. Pull requests never do. No gate or test runs there (R-005); run `bun run gates` on your machine before you merge.
2. The `version` job runs only on `main` and fails at once if `plugins.updater.pubkey` is still `REPLACE_WITH_OPERATOR_PUBLIC_KEY` or the `TAURI_SIGNING_PRIVATE_KEY` environment secret is empty. The version is `0.1.<run number>`; nothing is committed.
3. `build-macos` (universal app, ad-hoc signed unless every Apple secret is set) and `build-linux` (unsigned AppImage, then `sign-linux` signs it) run in parallel. macOS also checks `codesign --verify --deep --strict` and that `lipo -archs` gives `x86_64` and `arm64`.
4. `publish` runs when the version job and at least one build passed. It lists only the platforms that built in this run in `latest.json`, adds `Anima.dmg`, `Anima-x86_64.AppImage` and `SHA256SUMS`, creates the release as a draft, uploads every file with `latest.json` last, then publishes it as "latest".
5. Installed apps read `https://updates.anima.ing/latest.json`, which redirects to the latest release.

## Once, by the operator

Agents never do these steps (UPD-012). Steps marked 🔒 touch a secret: never paste the value into chat, a commit or a log.

**0. The branch.** The repository's default branch is not `main` today (it is `claude/extensions-planning-c47471`). The push trigger fires only when `main` exists and holds `release.yml`. A manual "Run workflow" from the UI needs the file on the default branch, and you must pick `main` as the branch: the `version` job refuses any other ref (UPD-007).

**a. 🔒 Make the update key pair** (file `~/.tauri/anima-updater.key`, public half `~/.tauri/anima-updater.key.pub`). Use the repository's own CLI. Do not run `tauri` through bunx or npx: that name resolves to an old unrelated npm package.

```sh
bun run --cwd apps/desktop tauri signer generate -w ~/.tauri/anima-updater.key
chmod 600 ~/.tauri/anima-updater.key
```

Back up the private key and its password offline: attach the key file to an entry in your password manager and store the password in the same entry, then keep an encrypted copy on a USB stick in a safe place. If you lose them, every installed app must be installed again by hand.

**b. Public key into the app.** Put the text of `~/.tauri/anima-updater.key.pub` in `apps/desktop/src-tauri/tauri.conf.json` as `plugins.updater.pubkey`, and commit it. The public key is not a secret.

**c. The `release` environment.** Do this BEFORE `main` exists or `release.yml` reaches it: GitHub creates an unprotected environment on first use. In the repository settings, create an environment named `release` and set its deployment branch policy to `main` only. All five jobs (`version`, both builds, `sign-linux` and `publish`) use it, so a run from another branch cannot read the secrets.

**d. 🔒 Environment secrets** (file `~/.tauri/anima-updater.key`). Store them in the `release` environment, not as repository secrets:

```sh
gh secret set --env release TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/anima-updater.key
gh secret set --env release TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # only if you set a password; it prompts
```

**e. Cloudflare redirect (DNS-007).** In the `anima.ing` zone:

1. Add a proxied DNS record for `updates` (for example an `AAAA` record for `100::`). The proxy must be on (orange cloud), or the redirect rule does not run.
2. Add one Redirect Rule (dynamic) for host `updates.anima.ing`, status 302, only for these four paths. Filter expression: `http.host eq "updates.anima.ing" and http.request.uri.path in {"/latest.json" "/Anima.dmg" "/Anima-x86_64.AppImage" "/SHA256SUMS"}`. Target: `concat("https://github.com/TheArctesian/anima/releases/latest/download", http.request.uri.path)`. Any other path gets a Cloudflare error page, not a 404, unless you add a rule that answers 404. Turn "Preserve query string" off. Turn "Normalize incoming URLs" on.
3. Check:

```sh
curl -sI https://updates.anima.ing/latest.json
```

It must show `HTTP/2 302` and a `location:` header with `.../releases/latest/download/latest.json`.

**f. 🔒 Optional: Developer ID signing and notarization.** Set all six as `release` environment secrets, or none (a partial set gives the ad-hoc signature): `APPLE_CERTIFICATE` (base64 `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password), `APPLE_TEAM_ID`. Set each with `gh secret set --env release NAME`.

**g. Keep the release path hard to attack.**
- Turn on immutable releases for the repository, so a published release cannot be changed.
- Turn on two-factor authentication on GitHub and on Cloudflare.
- Keep every action pinned by full commit SHA (a test enforces it). Update a pin only after you read the new code.

**h. First release.**
0. If any workflow run has already happened, run `gh cache delete --all` first, so no old cache is restored.
1. Create `main` from a `phase-1` commit whose gates passed (and do step c first).
2. Watch the run in the Actions tab.
3. Check `curl -sL https://updates.anima.ing/latest.json`: the version is `0.1.<n>` and both platforms are listed.
4. Install the first build by hand. An ad-hoc signed, quarantined `.app` needs System Settings > Privacy & Security > "Open Anyway", or `xattr -dr com.apple.quarantine /Applications/Anima.app`. The AppImage needs `chmod +x`.

**i. Acceptance item 16.**
1. A second merge to `main` produces `v0.1.<n+1>`.
2. The installed app shows "Anima 0.1.<n+1> is installed" within 4 hours, or at once through Settings > "Check for updates".
3. "Restart to update" runs the new version, and Settings shows it.
4. Test a bad signature without touching real releases. Build locally with a release `tauri build` (a debug build never checks) and a throwaway `--config` whose updater endpoint is a local https feed with a signature that does not match. The feed needs a certificate the system trusts. The app logs "Update install failed." and keeps running. The log is `~/Library/Logs/ing.anima.desktop/Anima.log` on macOS and `$XDG_DATA_HOME/ing.anima.desktop/logs` on Linux.

**Keychain prompts.** With ad-hoc signing, each update has a new code hash, so macOS will probably ask again for Keychain access to the stored token after each update. Developer ID signing (step f) avoids that.

## Signing keys and secrets in the workflow

- **macOS.** The build compiles with no secret in the environment (`tauri build --no-bundle`). The next step (`tauri bundle`) holds the signing key and the Apple secrets, because Tauri signs inside that command.
- **Linux.** The build job never sees the key. `tauri bundle --no-sign` downloads linuxdeploy with no hash check, then uploads the unsigned AppImage as `linux-unsigned`. The job `sign-linux` runs on a fresh runner (every `setup-bun` step sets `no-cache: true` and no job uses `actions/cache`, so nothing the build wrote is restored), installs only the Tauri CLI that the lockfile pins (`bun install --frozen-lockfile --ignore-scripts --filter @anima/desktop`: exact versions, no install scripts, only the desktop workspace), and runs `tauri signer sign --app-version` on the AppImage as data. Linux is listed in the feed only when `sign-linux` succeeded.
- **Residual risk on macOS.** The split stops only direct reads of the environment during the compile. The compile and the key step share one runner, so build-time code (`build.rs`, proc-macros, vite plugins, npm and Cargo dependencies) could change `PATH` or `node_modules` for the key step. On macOS, treat the build-time dependencies as trusted as much as the key. Developer ID signing and an external signer would reduce this later.

## What protects the feed

- Before the draft release is created, `make-feed` decodes each `.sig` and fails the run unless its trusted comment says `version:<this version>`. The apps use `requireSignedVersion`, so a `.sig` with no version or the wrong one would make every installed app refuse the update with no message.
- With `allowDowngrades: true`, `requireSignedVersion` stops mislabelled versions, not rollbacks. Anyone who controls the feed (`updates.anima.ing`, the Cloudflare account or the GitHub release) can send apps back to any older signed release. The protection is TLS and who controls those accounts, so keep two-factor authentication on both.
- Every hop of the DNS-007 redirect must be https. The updater follows an https-to-http redirect and only warns. The Cloudflare rule targets `https://github.com/...` only; check the `location:` header after any change.

## Known issue

If linuxdeploy fails with a `strip` error on `ubuntu-24.04`, set `NO_STRIP: true` on the Linux bundle step. It is not set today because the failure has not been seen.

## Roll back

Each release's feed points at its own files, so an older release works as it was. Mark it "latest":

```sh
gh release edit v0.1.<n> --latest
```

The apps run with `allowDowngrades` (set in `tauri.conf.json` under `plugins.updater`, not in the workflow), so at the next check they install that version. Do not delete the bad release until the rollback is in place.

## If a run fails

- A failed build leaves its platform out of that release; the other platform still publishes. Run the workflow again for a full release.
- If `publish` fails after the draft exists, delete the draft (`gh release delete v0.1.<n> --cleanup-tag`) and run the workflow again. A draft is not visible to the apps.
