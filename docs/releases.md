# Desktop releases

Spec: Section 3.9 (UPD-001 to UPD-012) and DNS-007. The workflow is `.github/workflows/release.yml`.

## How a release happens

1. A push to `main` (or "Run workflow" by hand) starts the workflow. Pull requests never do. No gate or test runs there (R-005); run `bun run gates` on your machine before you merge.
2. The `version` job runs only on `main` and fails at once if `plugins.updater.pubkey` is still `REPLACE_WITH_OPERATOR_PUBLIC_KEY` or the `TAURI_SIGNING_PRIVATE_KEY` environment secret is empty. The version is `0.1.<run number>`; nothing is committed.
3. `build-macos` (universal app, ad-hoc signed unless every Apple secret is set) and `build-linux` (AppImage) run in parallel. macOS also checks `codesign --verify --deep --strict` and that `lipo -archs` gives `x86_64` and `arm64`.
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

**c. The `release` environment.** Do this BEFORE `main` exists or `release.yml` reaches it: GitHub creates an unprotected environment on first use. In the repository settings, create an environment named `release` and set its deployment branch policy to `main` only. All four jobs (`version`, both builds and `publish`) use it, so a run from another branch cannot read the secrets.

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

## Signing keys and secrets in the workflow

Each build compiles with no secret in the environment (`tauri build --no-bundle`: vite, npm packages, `build.rs` and proc-macros run here). On macOS the next step (`tauri bundle`) holds the signing key and the Apple secrets. On Linux, `tauri bundle --no-sign` downloads linuxdeploy with no hash check, so it holds no key; a separate step runs `tauri signer sign --app-version` on the AppImage with only the updater key.

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

The apps run with `allowDowngrades` (set in the app's updater code, not in the workflow), so at the next check they install that version. Do not delete the bad release until the rollback is in place.

## If a run fails

- A failed build leaves its platform out of that release; the other platform still publishes. Run the workflow again for a full release.
- If `publish` fails after the draft exists, delete the draft (`gh release delete v0.1.<n> --cleanup-tag`) and run the workflow again. A draft is not visible to the apps.
