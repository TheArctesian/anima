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

Two paths make a release. The first is the Builds view of a hub that holds the updater key: the paragraphs below and the section "Once, by the operator" describe it, and the operator does not use that hub now (R-175). The second is the release workflow of GitHub Actions (R-179), which is built and not yet run: see "The release workflow".

On the first path a release starts only from the Builds view (BLD-001, UPD-007). The operator's builders build each
target and upload the unsigned files to the R2 bucket `anima-releases`. The hub checks them, signs
the updater file with the updater key it holds, writes `<file>.sig` beside it and then
`https://releases.anima.ing/<target>-<arch>/latest.json` (BLD-064 to BLD-067, R-117). Installed apps read
that feed. The hub side, its R2 credentials and the bucket setup are in
[deploy/README.md](../../deploy/README.md).

The old GitHub release workflow was removed by R-114; the file `.github/workflows/release.yml` of
today is a new one (R-171, R-179). On the first path no
builder holds the updater signing key; it is a secret file of the hub (R-117). A
compromised hub can therefore sign updates that installed apps trust; the operator accepted this
in R-117.

The operator also keeps a `release` environment on GitHub with the Apple secrets and a copy of
the updater key, `TAURI_SIGNING_PRIVATE_KEY` (R-116). The release workflow reads them, from three
environments: "The release workflow" has the list.

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

## The release workflow (R-179, R-180)

The workflow is on `main`. HT-036 of [need-human-testing.md](../need-human-testing.md) holds the check of the first run. The [projects and builds plan](../plans/projects-and-builds-plan.md) owns the workflow design and its open decisions. This section gives the operator's steps.

`.github/workflows/release.yml` builds, signs and publishes a release on machines of GitHub, with no hub (R-173: the job signs; that the workflow also publishes is OD-W33, open). A tag `v<version>` on a commit of `main` starts a release. A run by hand is a trial and publishes nothing. The jobs `sign` and `publish` run `anima-hub release`, which makes the checks that the hub makes for a job of the Builds view.

### Before the first run, by the operator

Agents never do these steps (R-180, UPD-012). Never put a key or a password in a chat, a commit or a log. See the inventory below for the secret names already present on GitHub.

1. **Who can write.** Settings > Collaborators and teams: only your account has the Write or Admin role. Each account with write access can put a tag and can thus ask for a release.
2. **Workflows from forks.** Settings > Actions > General > "Approval for running fork pull request workflows from contributors": select "Require approval for all external contributors". A workflow that a pull request from a fork starts gets no secret of the repository in a public repository, and the release workflow has no pull request trigger. On the same page, "Workflow permissions": "Read repository contents and packages permissions".
3. **The tag rule.** Settings > Rules > Rulesets > New ruleset > New tag ruleset. Name: `release tags`. Enforcement: Active. Target tags: the pattern `v*`. Rules: "Restrict creations", "Restrict updates", "Restrict deletions". Bypass list: "Repository admin" only. Then only you can put a tag `v…`, and nobody can move one.
4. **The branch rule for `main`.** It is necessary only if a second account can write. If so: a branch ruleset for `main` with "Restrict deletions", "Block force pushes" and "Require a pull request before merging". The workflow builds only a commit of `main`, so a person who can change `main` can change what is released.
5. **The environment `release`** (the macOS build). Settings > Environments > `release` (or "New environment"). "Required reviewers": your account; leave "Prevent self-review" off, because you put the tag and you approve. "Allow administrators to bypass configured protection rules": off. "Deployment branches and tags": "Selected branches and tags", with the tag rule `v*` and the branch rule `main` (a trial runs from `main`). R-116 says that this environment allows `main` only: add the tag rule. Environment secrets:

   | Secret | Value form |
   | --- | --- |
   | `APPLE_CERTIFICATE` | The Developer ID Application certificate with its private key, exported from Keychain Access as a `.p12` file, in base64 on one line |
   | `APPLE_CERTIFICATE_PASSWORD` | The password that you gave the `.p12` file |
   | `APPLE_ID` | The Apple account of the notary service |
   | `APPLE_PASSWORD` | An app-specific password of that account (`xxxx-xxxx-xxxx-xxxx`), made at account.apple.com |
   | `APPLE_TEAM_ID` | `7GDLG48PV6` (it is in `tauri.conf.json`; it is not a secret) |

   Delete `TAURI_SIGNING_PRIVATE_KEY` from this environment after step 6: the macOS build must not have it.
6. **The environment `release-sign`** (the updater key). New environment, with the same reviewer and the same two rules (`v*` and `main`). Environment secrets:

   | Secret | Value form |
   | --- | --- |
   | `TAURI_SIGNING_PRIVATE_KEY` | The text of `~/.config/anima/updater.key`: one line of base64 |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password of the key. Make no such secret if the key has no password |

   A command that sends the file with no copy on the screen or in the clipboard (you run it; it needs `gh auth login` on your Mac):

   ```sh
   gh secret set TAURI_SIGNING_PRIVATE_KEY --env release-sign --repo TheArctesian/anima.ing < ~/.config/anima/updater.key
   ```
7. **The environment `release-publish`** (the bucket). New environment, with the same reviewer and the tag rule `v*` only: a trial never uses it. In the Cloudflare dashboard, R2 > Manage R2 API Tokens > Create API token: permission "Object Read & Write", "Apply to specific buckets only": `anima-releases`. It shows an Access Key ID and a Secret Access Key one time.

   | Name | Kind | Value form |
   | --- | --- | --- |
   | `R2_ACCESS_KEY_ID` | Secret | The Access Key ID of that token |
   | `R2_SECRET_ACCESS_KEY` | Secret | The Secret Access Key of that token |
   | `RELEASES_ENDPOINT` | Variable | `https://<account ID>.r2.cloudflarestorage.com` |
8. **The state of `main`.** The job `gates` runs `bun run gates` on the release commit. On 2026-11-18 `main` did not pass the limits gate (two files of `docs/ideas/browser-first-run/`), so a release from that commit stops at the gates.

### GitHub secret inventory

Checked on 2026-11-18 with `gh secret list --repo TheArctesian/anima.ing --env release`. GitHub lists the names and update times. The workflow must prove that it can use the stored values.

| Environment | Use | Secret names present |
| --- | --- | --- |
| `release` | macOS Developer ID signing and notarization | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` |
| `release` | Existing entries that the current workflow does not read from this environment | `APPLE_SIGNING_IDENTITY`, `TAURI_SIGNING_PRIVATE_KEY` |
| `release` | iOS app signing | `IOS_DISTRIBUTION_P12_BASE64`, `IOS_DISTRIBUTION_P12_PASSWORD`, `IOS_PROVISIONING_PROFILE_BASE64` |
| `release` | App Store Connect upload | `ASC_API_PRIVATE_KEY`, `ASC_API_KEY_ID`, `ASC_API_ISSUER_ID` |

The iOS `.p12` holds the Apple Distribution certificate and its private key. The profile is the App Store Connect profile for `ing.anima.mobile`. The operator verified the export password against the `.p12` before setting that secret. The upload key is an App Store Connect team key with the Developer role. The iOS job is pending.

Only `release` existed when checked. Steps 6 and 7 give the required entries for `release-sign` and `release-publish`. No Android signing secret was present.

### A trial that publishes nothing

1. Actions > release > "Run workflow". Branch: `main`. Version: `0.0.1`. Use a version that you will never release: the trial makes a real signed and notarized application and a real updater signature for the version that you give, and a person who is signed in to GitHub can download the files of a run of a public repository for 7 days.
2. Approve the environment `release` when the run asks (the macOS build). When the builds and the gates have passed, approve `release-sign`.
3. The run passes when the job `trial` passes. The job `publish` is skipped. Nothing is in the bucket and no feed changed. The application of a trial run is the application that a release makes: it has the updater. So when you start it, it reads the public feed and offers the published version, which is higher than `0.0.1`; that is correct, and it shows that the updater is on. Settings > Updates must not say "This is a local build".
4. From the page of the run, download the artifacts `macos` and `linux`. On a Mac:

   ```sh
   xcrun stapler validate Anima_0.0.1_universal.dmg
   spctl --assess --type open --context context:primary-signature --verbose Anima_0.0.1_universal.dmg
   ```

   Open the DMG, then `spctl --assess --type execute --verbose "/Volumes/Anima.ing/Anima.ing.app"` (expected: `accepted`, `source=Notarized Developer ID`). On a Linux computer with `x86_64`: `chmod +x Anima_0.0.1_amd64.AppImage` and start it (OD-W38: no AppImage of Anima was checked on a Linux computer before).

### The first real release (0.2.2)

1. Keep the three feeds of today, for a rollback:

   ```sh
   for feed in darwin-aarch64 darwin-x86_64 linux-x86_64; do curl -fsS -o "feed-$feed.json" "https://releases.anima.ing/$feed/latest.json" || echo "$feed has no feed"; done
   ```
2. Put the tag on the commit of `main` that you release, and push only the tag:

   ```sh
   git fetch origin
   git tag -a v0.2.2 -m "Anima 0.2.2" origin/main
   git push origin refs/tags/v0.2.2
   ```
3. Approve `release`, then `release-sign`, then `release-publish`, each when the run asks. The page of the approval shows the tag and the commit: approve only your own tag. The last approval is the point after which the release is public.
4. Check, after the job `publish` passed:
   - Each feed gives 0.2.2 (the job also reads them): `curl -fsS https://releases.anima.ing/darwin-aarch64/latest.json`, and the same for `darwin-x86_64` and `linux-x86_64`.
   - Download `https://releases.anima.ing/Anima.dmg` and run the two commands of the trial on it and on the application in it.
   - On a Mac with 0.2.1 installed: Settings > Updates offers 0.2.2, and "Restart to update" starts 0.2.2. DEF-001 (an application that a computer built replaced itself with 0.2.1) is corrected on a different branch; use an application that was installed from the DMG of 0.2.1 for this check.
5. Set the row of 0.2.2 in the release record at the top of this document, and the result of HT-036.

### To stop or undo a release

- **Before the job `publish`:** reject the approval of `release-publish`, or cancel the run. Nothing public changed. The files of the run stay on GitHub for 7 days.
- **While `publish` runs:** do not cancel it. If it fails between two feeds, select "Re-run failed jobs": the job keeps each file that the bucket already holds with the same bytes, and writes the same feed again.
- **After a bad release:** the feed does not go back by itself (BLD-069), and the workflow refuses a lower version. The usual way is a new version: correct `main`, and put the tag of the next version. To take the bad version away at once, write the feed files of step 1 back to the bucket by hand, one for each feed (R2 > `anima-releases` > the folder of the feed > upload `latest.json`, or `wrangler r2 object put anima-releases/darwin-aarch64/latest.json --file feed-darwin-aarch64.json --content-type application/json --cache-control no-cache`). An installed application then goes back to that version, because the updater permits a lower version (`allowDowngrades`). An application that already installed the bad version keeps it until it reads the feed again.
- **A version is used one time.** The workflow refuses to publish 0.2.2 again with other bytes. After a rollback the next release is 0.2.3. Do not move or delete a pushed tag.
- **A key that another person may hold:** delete the secret, then make a new key. For the updater key no remote step exists: each installed application trusts the old key until it is installed again by hand (UPD-012).

### What the operator accepts with this form

- The updater key that each installed application trusts, the Developer ID certificate and the bucket token are in the secrets of GitHub. GitHub the company can read them.
- An update can be signed by each person who can make the environment `release-sign` run a workflow: a person who can put a tag `v…` or change `main`, and get your approval or act as your account. The workflow file of the tagged commit is the one that runs, so a tag on a commit with a changed workflow can skip each check of this document.
- The protections are the ones of "Before the first run": only your account writes, only you put a tag `v…`, you approve each use of each key and you look at the tag and the commit when you approve, and each key is in an environment of its own so that the builds do not hold the updater key or the bucket token.
- The actions of GitHub that the workflow uses are pinned to a commit, but the machines, the images and the tools that they install (Homebrew, apt, `linuxdeploy`, the crates and packages of the lock files) are code of other people that runs in the jobs that build.
- Until OD-W34 and OD-W37 are decided, the hub on the VPS also holds a copy of the updater key.

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
an application on another Mac. A trial bundle does not update itself: it has no updater, and
Settings > Updates says "This is a local build. It does not update itself." (UPD-001). To get a
newer version, build it again or install a release.

The jobs `macos` and `linux` of the release workflow run the same script with
`ANIMA_RELEASE_BUILD=1`, and on macOS with `SIGN_IDENTITY` set to the identity of
`tauri.conf.json`. Only then the application has the updater and reads the public feed, as an
application of the Builds path does; by hand, with or without an identity, the script makes a
local build that does not update itself (DEF-001). A test holds the two modes
(`deploy/test/release-workflow.test.ts`). The job `macos` then makes the checks, the notary step
and the updater file itself. `apps/desktop/src-tauri/check-mach-o.sh <application> <Team ID>` is its check of each
Mach-O file (UPD-011); you can run it on an installed application, for example
`sh apps/desktop/src-tauri/check-mach-o.sh /Applications/Anima.ing.app 7GDLG48PV6`.

To try the commands of the jobs `sign` and `publish` on a computer, with a throwaway key and a
bucket that the test makes in its own process: `cargo nextest run -p anima-hub -E 'test(/publishing::built/)'`.
The rules of the workflow file have a test too: `bun test deploy/test/release-workflow.test.ts`
(from `deploy/`), and `actionlint .github/workflows/release.yml` reads the file.

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

Run it alone with `cargo test -p anima-app --test updater`.

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
