# Desktop releases

Spec: Section 3.9 (UPD-001 to UPD-012) and DNS-007. The workflow is `.github/workflows/release.yml`.

## How a release happens

1. A push to `main` (or "Run workflow" by hand) starts the workflow. Pull requests never do. No gate or test runs there (R-005); run `bun run gates` on your machine before you merge.
2. The `version` job fails at once if `plugins.updater.pubkey` is still `REPLACE_WITH_OPERATOR_PUBLIC_KEY` or the `TAURI_SIGNING_PRIVATE_KEY` secret is empty. The version is `0.1.<run number>`; nothing is committed.
3. `build-macos` (universal app, ad-hoc signed unless every Apple secret is set) and `build-linux` (AppImage) run in parallel. macOS also checks `codesign --verify --deep --strict` and that `lipo -archs` gives `x86_64` and `arm64`.
4. `publish` runs when the version job and at least one build passed. It lists only the platforms that built in this run in `latest.json`, adds `Anima.dmg`, `Anima-x86_64.AppImage` and `SHA256SUMS`, creates the release as a draft, uploads every file with `latest.json` last, then publishes it as "latest".
5. Installed apps read `https://updates.anima.ing/latest.json`, which redirects to the latest release.

## Once, by the operator

Agents never do these steps (UPD-012). Steps marked 🔒 touch a secret: never paste the value into chat, a commit or a log.

**a. 🔒 Make the update key pair** (file `~/.tauri/anima-updater.key`, public half `~/.tauri/anima-updater.key.pub`):

```sh
bunx tauri signer generate -w ~/.tauri/anima-updater.key
```

Back up the private key and its password offline. If you lose them, every installed app must be installed again by hand.

**b. Public key into the app.** Put the text of `~/.tauri/anima-updater.key.pub` in `apps/desktop/src-tauri/tauri.conf.json` as `plugins.updater.pubkey`, and commit it. The public key is not a secret.

**c. 🔒 GitHub secrets** (file `~/.tauri/anima-updater.key`):

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/anima-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # only if you set a password; it prompts
```

**d. Cloudflare redirect (DNS-007).** In the `anima.ing` zone:

1. Add a proxied DNS record for `updates` (for example an `AAAA` record for `100::`). The proxy must be on (orange cloud), or the redirect rule does not run.
2. Add a Redirect Rule for host `updates.anima.ing`, status 302:
   - `/latest.json` to `https://github.com/TheArctesian/anima/releases/latest/download/latest.json`;
   - every other path `/<file>` to the same file of the latest release, with the dynamic expression `concat("https://github.com/TheArctesian/anima/releases/latest/download", http.request.uri.path)`.
3. Check:

```sh
curl -sI https://updates.anima.ing/latest.json
```

It must show `HTTP/2 302` and a `location:` header with `.../releases/latest/download/latest.json`.

**e. 🔒 Optional: Developer ID signing and notarization.** Set all six secrets, or none (a partial set gives the ad-hoc signature): `APPLE_CERTIFICATE` (base64 `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password), `APPLE_TEAM_ID`. Set each with `gh secret set NAME`.

## Roll back

Each release's feed points at its own files, so an older release works as it was. Mark it "latest":

```sh
gh release edit v0.1.<n> --latest
```

The apps run with `allowDowngrades`, so at the next check they install that version. Do not delete the bad release until the rollback is in place.

## If a run fails

- A failed build leaves its platform out of that release; the other platform still publishes. Run the workflow again for a full release.
- If `publish` fails after the draft exists, delete the draft (`gh release delete v0.1.<n> --cleanup-tag`) and run the workflow again. A draft is not visible to the apps.
