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

The operator keeps three environments on GitHub: `release` with the Apple secrets (R-116),
`release-sign` with the updater key, and `release-publish` with the bucket token. The release
workflow reads them: "The release workflow" has the list.

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

## The release workflow (R-179, R-180, R-182 to R-184, R-190 to R-195, R-198 to R-204)

The workflow is on `main`. HT-036 of [need-human-testing.md](../need-human-testing.md) holds the check of its runs. The [projects and builds plan](../plans/projects-and-builds-plan.md) owns the workflow design and its open decisions. This section gives the state of GitHub and the operator's steps.

`.github/workflows/release.yml` builds, signs and publishes a release on machines of GitHub, with no hub (R-173: the job signs; R-191: the workflow publishes; R-190: it runs the gates first).

**Each push to `main` can be a release (R-184, R-198, R-199).** The first job of a run compares the pushed commit with the version that each feed gives now. If a path of the desktop application changed after that version, the run builds, signs and publishes the version of the pushed commit. If none changed, the run ends after that one job, and its summary says "nothing" with the reason. `deploy/release/applications.txt` is the map of the paths; each change inside the folders of the application counts, tests and comments too (R-200). A run by hand is a trial that publishes nothing. No tag and no pull request starts the workflow.

**No approval (R-182).** No person approves a run. To bring `dev` into `main` is the act that releases (R-198).

**The switch.** While the repository variable `ANIMA_PUBLIC_RELEASES` is not `on`, a push to `main` builds nothing and publishes nothing; the summary of the run says what it would release. The variable does not exist today, so public releases are off. "To switch public releases on" below has the steps.

### The version of a release (R-183, R-201)

The version of a commit of `main` is `0.<N div 100>.<N mod 100>`, where N is the number of commits on the first line of its history (the own line of `main`; a branch that is merged adds one commit, its merge). This command prints it:

```sh
sh deploy/bin/commit-version origin/main
```

On 2026-11-18 the first line of `main` had 800 commits, which gives 0.8.0; it is more than the published 0.2.1. No person writes a version, and no file holds one. A build by hand with `trial-bundle.sh` and no version gets the version of the commit that is checked out: on `dev` that is a number of the line of `dev`, which a commit of `main` can also have (R-202). Such a build does not update itself. The command refuses a shallow clone.

**The condition (R-201).** `main` must move only by a commit whose first parent is the tip of `main` before it: a merge of `dev` with `--no-ff`, or a commit directly on `main`. A fast-forward of `main` to a branch can make the count fall or jump. The first job of a run fails, with no release, when the tip of `main` before the push is not on the first line of the pushed commit.

### The branch `dev` and the act that releases (R-198)

The daily work goes to the branch `dev`. Agents start from `origin/dev`, merge into `dev` and push their claims to `dev`; they do not push to `main` (AGENTS.md). `main` holds what is released.

To make `dev` exist, one time (the operator, or a session that he tells to do it):

```sh
git fetch origin && git push origin origin/main:refs/heads/dev
```

To release what `dev` holds: a merge commit on `main`, never a fast-forward.

```sh
git fetch origin
git switch --detach origin/main
git merge --no-ff origin/dev -m "Release: dev into main"
git push origin HEAD:refs/heads/main
```

The push is the release, when the switch is on. `dev` needs no merge back: the next merge of `dev` into `main` still works.

Rules of GitHub that the operator should set for `main` (Settings > Rules > Rulesets > New branch ruleset, target `main`): "Restrict deletions" and "Block force pushes". They keep the first line of `main`. Do not select "Require linear history": it forbids the merge commit that the version needs. The rule set `release tags` is no longer used by the workflow; it can stay.

What each other stream that pushes to `main` today must be told (Codex, and each coding agent with rules of its own): (1) start from `origin/dev` and merge into `dev`; (2) never push to `main`: a push to `main` is a public release; (3) never fast-forward `main`; (4) the base of a gates run does not change (`BASE_REF`, default `phase-2`).

### The state of GitHub

Read from GitHub on 2026-11-18 (evening; 2026-11-19 UTC) with `gh`. The operator, or the orchestrating session on his instruction (R-195), set each item. Never put a key or a password in a chat, a commit or a log.

| Item | State |
| --- | --- |
| Who can write | One account, the operator's |
| Workflows of outside contributors | Approval required for all external contributors |
| Workflow permissions | Read only |
| Tag rule set `release tags` | Active. Pattern `v*`. Creations, updates and deletions are restricted; force pushes are blocked. Only a repository administrator can bypass it |
| Environment `release` | Permits the branch `main` and tags `v*`. No required reviewer. Administrators can bypass |
| Environment `release-sign` | Permits the branch `main` and tags `v*`. No required reviewer. Administrators can bypass |
| Environment `release-publish` | Permits tags `v*` only; a run of `main` cannot publish until the branch `main` is added ("To switch public releases on"). No required reviewer. Administrators can bypass |

The secrets and variables. GitHub lists names and times, never values. A run must prove that each value is usable.

| Environment | Use | Names |
| --- | --- | --- |
| `release` | macOS Developer ID signing and notarization (job `macos`) | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` |
| `release` | Not read by the workflow | `APPLE_SIGNING_IDENTITY` |
| `release` | iOS app signing and App Store Connect upload; the iOS job is not built | `IOS_DISTRIBUTION_P12_BASE64`, `IOS_DISTRIBUTION_P12_PASSWORD`, `IOS_PROVISIONING_PROFILE_BASE64`, `ASC_API_PRIVATE_KEY`, `ASC_API_KEY_ID`, `ASC_API_ISSUER_ID` |
| `release-sign` | The updater key (job `sign`) | `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |
| `release-publish` | The token of the bucket `anima-releases` (job `publish`) | `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, and the variable `RELEASES_ENDPOINT` |
| `release-publish` | Not read by the workflow | `ACCOUNT_ID`, `S3_API`, `TOKEN_VALUE` |

The operator deleted `TAURI_SIGNING_PRIVATE_KEY` from `release`, so the macOS build does not have the updater key. The orchestrating session set the two `R2_…` secrets from a file of the operator, on his instruction, and printed no value (R-195). The three names that the workflow does not read in `release-publish`, and `APPLE_SIGNING_IDENTITY`, can be deleted; a job gets each secret of its environment in memory, also one that no step names. The iOS secrets are in the environment of the macOS build for the same reason: when the iOS job is built, give them an environment of their own.

The iOS `.p12` holds the Apple Distribution certificate and its private key. The profile is the App Store Connect profile for `ing.anima.mobile`. The operator verified the export password against the `.p12` before he set that secret. The upload key is an App Store Connect team key with the Developer role. No Android signing secret is present.

The value forms, for a secret that must be made again:

| Name | Value form |
| --- | --- |
| `APPLE_CERTIFICATE` | The Developer ID Application certificate with its private key, exported from Keychain Access as a `.p12` file, in base64 on one line |
| `APPLE_CERTIFICATE_PASSWORD` | The password of that `.p12` file |
| `APPLE_ID` | The Apple account of the notary service |
| `APPLE_PASSWORD` | An app-specific password of that account (`xxxx-xxxx-xxxx-xxxx`), made at account.apple.com |
| `APPLE_TEAM_ID` | `7GDLG48PV6` (it is in `tauri.conf.json`; it is not a secret) |
| `TAURI_SIGNING_PRIVATE_KEY` | The text of `~/.config/anima/updater.key`: one line of base64 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password of the key. No such secret if the key has no password |
| `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | The two parts of an R2 API token with "Object Read & Write" for the bucket `anima-releases` only |
| `RELEASES_ENDPOINT` (variable) | `https://<account ID>.r2.cloudflarestorage.com` |

A command that sends a file as a secret with no copy on the screen or in the clipboard (it needs `gh auth login`):

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY --env release-sign --repo TheArctesian/anima.ing < ~/.config/anima/updater.key
```

If a second account gets write access: add "Require a pull request before merging" to the branch rule set of `main` ("The branch `dev` and the act that releases"). A person who can change `main` releases.

### A trial that publishes nothing

1. Actions > release > "Run workflow". Branch: `main`. Version: `0.0.1`. Do not give the version of a release: the trial makes a real signed and notarized application and a real updater signature for the version that you give, and a person who is signed in to GitHub can download the files of a run of a public repository for 7 days.
2. The run asks for no approval. It passes when the job `trial` passes. The job `publish` is skipped. Nothing is in the bucket and no feed changed. The application of a trial run is the application that a release makes: it has the updater. So when you start it, it reads the public feed and offers the published version, which is higher than `0.0.1`; that is correct, and it shows that the updater is on. Settings > Updates must not say "This is a local build".
3. From the page of the run, download the artifacts `macos` and `linux`. On a Mac:

   ```sh
   xcrun stapler validate Anima_0.0.1_universal.dmg
   spctl --assess --type open --context context:primary-signature --verbose Anima_0.0.1_universal.dmg
   ```

   Open the DMG, then `spctl --assess --type execute --verbose "/Volumes/Anima.ing/Anima.ing.app"` (expected: `accepted`, `source=Notarized Developer ID`). On a Linux computer with `x86_64`: `chmod +x Anima_0.0.1_amd64.AppImage` and start it. The operator says that an older AppImage of Anima ran on Linux; this check is for the AppImage of today, and the first public release waits for it (R-192).

### To switch public releases on

Do this only when the trial has passed and the AppImage of the trial has started on a Linux computer (R-192). Two steps, both by the operator; an agent does neither (R-180, R-195).

1. Let the environment `release-publish` take a run of `main` (it permits only tags `v*` today): Settings > Environments > `release-publish` > "Deployment branches and tags" > "Add deployment branch or tag rule" > branch `main`. Or:

   ```sh
   gh api --method POST repos/TheArctesian/anima.ing/environments/release-publish/deployment-branch-policies -f name=main -f type=branch
   ```
2. Set the switch:

   ```sh
   gh variable set ANIMA_PUBLIC_RELEASES --body on --repo TheArctesian/anima.ing
   ```

To switch them off again: `gh variable delete ANIMA_PUBLIC_RELEASES --repo TheArctesian/anima.ing`. To stop a run that already started, cancel it on GitHub.

### The first public release

It waits for the two conditions above, and nothing makes it urgent before the LLC exists. It carries macOS and Linux. Its version is the version of its commit (R-183, R-201).

1. Keep the three feeds of today, for a rollback:

   ```sh
   for feed in darwin-aarch64 darwin-x86_64 linux-x86_64; do curl -fsS -o "feed-$feed.json" "https://releases.anima.ing/$feed/latest.json" || echo "$feed has no feed"; done
   ```
2. Switch public releases on. The switch alone releases nothing: a run starts only from a push.
3. Push to `main` (the merge of `dev`, above). The Linux feed does not exist, so the first push after the switch is a release, also when it changes only documents.
4. Check, after the job `publish` passed:
   - Each feed gives the version (the job also reads them): `curl -fsS https://releases.anima.ing/darwin-aarch64/latest.json`, and the same for `darwin-x86_64` and `linux-x86_64`.
   - Download `https://releases.anima.ing/Anima.dmg` and run the two commands of the trial on it and on the application in it.
   - On a Mac with 0.2.1 installed from its DMG: Settings > Updates offers the version, and "Restart to update" starts it. An application that a computer built does not update itself (DEF-001).
5. Set the row of the version in the release record at the top of this document, and the result of HT-036.

### To stop or undo a release

- **Before the job `publish`:** cancel the run on GitHub, or delete the variable `ANIMA_PUBLIC_RELEASES`. Nothing public changed. The files of the run stay on GitHub for 7 days. No approval stops a run (R-182): if you do neither, it publishes.
- **While `publish` runs:** do not cancel it. If it fails between two feeds, select "Re-run failed jobs": the job keeps each file that the bucket already holds with the same bytes, and writes the same feed again. If you do not, the next push that is a release publishes both targets with its own version: the feed that is behind decides.
- **After a bad release:** the feed does not go back by itself (BLD-069), and the workflow releases nothing for a version that is not higher. The usual way is a new commit on `main`: a revert is a new commit with a higher version. To take the bad version away at once, write the feed files of step 1 back to the bucket by hand, one for each feed (R2 > `anima-releases` > the folder of the feed > upload `latest.json`, or `wrangler r2 object put anima-releases/darwin-aarch64/latest.json --file feed-darwin-aarch64.json --content-type application/json --cache-control no-cache`). An installed application then goes back to that version, because the updater permits a lower version (`allowDowngrades`). An application that already installed the bad version keeps it until it reads the feed again. After such a step the next push compares with the old version in the feed, so it releases again.
- **A version is used one time.** The workflow refuses to publish a version again with other bytes. A commit has one version, so a release that is published wrong is corrected by a later commit.
- **Two pushes near each other:** releases run one at a time, in the order of the pushes. GitHub keeps one run that waits, and cancels an older one that waits; the run that comes next carries its changes. A cancelled run in the list is thus normal.
- **A run that fails with "main did not move by a commit whose first parent is its tip before":** `main` was fast-forwarded or pushed by force. Nothing was released. Look at `git log --first-parent origin/main`, and make the next move of `main` a merge commit; if the count fell below the published version, releases start again when the first line is longer than that.
- **A key that another person may hold:** delete the secret, then make a new key. For the updater key no remote step exists: each installed application trusts the old key until it is installed again by hand (UPD-012).

### What the operator accepts with this form

- The updater key that each installed application trusts, the Developer ID certificate and the bucket token are in the secrets of GitHub. GitHub the company can read them.
- No person approves a release (R-182), and each push to `main` that changes the application is one (R-198, R-199). An update is signed and published for each person or program that can push to `main` as your account: your account, a token of it, and each coding agent that runs with your `gh` or git credentials on your computer. The rule that agents use `dev` is a rule of AGENTS.md; GitHub does not hold an agent to it, because the agent acts as you. The workflow file of the pushed commit is the one that runs, so a push with a changed workflow can skip each check of this document, the switch also.
- The protections that stay: only your account writes; the switch `ANIMA_PUBLIC_RELEASES`; each environment permits only named branches and tags; each key is in an environment of its own, so the builds do not hold the updater key or the bucket token; the gates run before the job that signs (R-190); no pull request starts the workflow.
- The actions of GitHub that the workflow uses are pinned to a commit, but the machines, the images and the tools that they install (Homebrew, apt, `linuxdeploy`, the crates and packages of the lock files) are code of other people that runs in the jobs that build.
- The hub on the VPS also holds a copy of the updater key, and the Builds path can still sign through it. The operator left it for now (R-194). One key is in two places until he decides.
- The secrets that the workflow does not read stay where they are (R-204): `APPLE_SIGNING_IDENTITY` and the six iOS secrets in `release`, and `ACCOUNT_ID`, `S3_API` and `TOKEN_VALUE` in `release-publish`. The job of each environment has them in its memory.
- The workflow puts no tag. The record of a release is the feed, the folder `<target>/<version>/` of the bucket and the release record of this document; the commit of a version is the commit at that position on the first line of `main`. A tag by the workflow would need a write permission on the repository in a job, and the rule set `release tags` permits a tag `v…` only to an administrator.
- The standard runners of GitHub cost nothing for a public repository (R-193 has the source). If the repository becomes private, each macOS minute is charged.

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
that runs it, with the version of the commit that is checked out when you give none (R-183), with an ad hoc signature on macOS and no notary step. Use it to try the hub from
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
