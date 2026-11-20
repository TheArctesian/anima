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

### Windows builds and releases (WIN2)

The `gates` workflow calls `.github/workflows/windows-trial.yml` for `dev` and each merge
train, and refuses a train whose native checks fail. This also builds artifacts when the
merge train advances dev with GitHub's workflow token (which does not trigger push workflows).
The Actions gates run produces
`windows-trial` (the per-user NSIS installer, SHA256SUMS and source commit) and
`windows-trial-cli` (the standalone command). Download and extract the artifact, then run
`Anima_0.0.1_x64-setup.exe`. These trials are unsigned and have automatic updates disabled.
Run the existing `gates` workflow manually on `dev` to make a new trial. After the new
workflow is on the default branch, it can also be run directly by hand. Artifacts last 14 days.

The Windows job builds on `windows-2025` with MSVC, runs all native gates, checks compiler
discovery, installs the package silently and compares all seven installed sidecars with
the packaged files. It also checks the installed CLI version and PostgreSQL payload.
`windows-gates` keeps the native gate log even when a trial fails.

The public `release` workflow builds the same package at the version of `main`. Its
Windows job requires Authenticode signing; the installer and sidecars must have a valid,
timestamped publisher signature. The isolated `sign` job then adds the Tauri updater
signature after all platform jobs pass. The publish job checks hashes and signatures
before it writes the versioned files, then writes `windows-x86_64/latest.json` last.
Windows participates in the release decision, including recovery from a missing feed.

Before a signed trial, the operator configures environment `release-windows` with
`WINDOWS_CERTIFICATE` (base64 PFX containing one code-signing private key) and
`WINDOWS_CERTIFICATE_PASSWORD`. Restrict that environment to `main`. The runner imports
the PFX into its current-user certificate store, removes the temporary file immediately,
and deletes the signing key at job end. A missing, expired, untrusted or untimestamped
certificate fails the build; it never falls back to an unsigned public installer.
Use a certificate whose provider permits this export/import method. A hardware-backed or
cloud-only signing identity needs its provider integration before a public release.
The configuration follows [Tauri's Windows signing procedure](https://v2.tauri.app/distribute/sign/windows/).
The existing `release-sign` updater key and `release-publish` bucket credentials remain
separate. No credentials or repository switches were changed by this implementation.

A manual `release` run on `main` builds and signs but publishes nothing. Check its installer
in the Windows VM, then exercise a private update from an older signed version before a
public release. The Windows acceptance entry in [need-human-testing.md](../need-human-testing.md)
records these checks. Merging into `dev` does not publish an update.

### The gates of a release (BLD-042, R-190)

A release signs only after the gates passed for its content. Three jobs do this, and none holds a secret.

| Job | Machine | What it does |
| --- | --- | --- |
| `gates-linux` | Linux, four jobs at the same time | Calls the workflow `gates` (`.github/workflows/gates.yml`, the one that tests each branch before its merge into `dev`) for the release commit: each of the 21 gates |
| `gates-macos` | `macos-15` | Runs `deploy/bin/macos-tests`: the tests that Linux skips or does not compile. They are the Mach-O check, the refusal of a release build with no signing identity, the access lists of `link/statedir`, the Rust tests of each package with code for macOS only (the desktop shell, the daemon, `anima`, the database tree, and the link tests of the hub), and the pinning test of the desktop shell |
| `gates` | Linux | Passes when the Mac tests passed, and the Linux gates passed or were not needed. The jobs `sign` and `ios-upload` wait for it |

**One content is not gated two times.** The work reaches `main` as a merge of `dev`, and the gates passed on `dev` for that content. The first job of a release asks GitHub for the runs of the workflow `gates` and looks for one that passed for the same content. If it finds one, `gates-linux` does not run; the summary of the run names the run that it found. `gates-macos` runs in each release.

The rule of `deploy/bin/gated-run`, in full. A run counts only when: it is a run of the file `.github/workflows/gates.yml` of this repository; for the branch `dev` of this repository; started by a push or by hand; complete with the result success, and its job `passed` has the result success; and the tree of its commit is the tree of the release commit. The tree holds each file, the gates and the workflows too, so the same gates ran on the same files. A run for a different branch never counts: it tests that branch as merged with `dev`, which is not the tree of its commit.

Why a branch cannot fake it: the list comes from GitHub, not from a file of the repository. Only a push to `dev` or a run by hand on `dev` adds a run that counts, and such a run tests the commit of `dev` with the workflow file of `dev`. A person who can push to `dev` can also push to `main`, so this adds no new power. If the list cannot be read, or anything in it is not as expected, the gates run.

When `main` gets a commit whose content was never on `dev`, no run counts and `gates-linux` runs. It then has no compile cache: the caches of the workflow `gates` are written by runs of `dev`, and a run of `main` cannot read them. Expect more than the 31 minutes of a run on `dev`.

To run the Mac tests on a Mac: `sh deploy/bin/macos-tests`, with PostgreSQL 17, cargo-nextest and Go on `PATH`.

Do not start a trial for the commit of a release that is still running, and the other way round: the workflow `gates` cancels an older run of itself for the same commit, so the first of the two would lose its gates and fail. Nothing is published in that case; start the failed run again.

### The version of a release (R-183, R-201)

The version of a commit of `main` is `0.<N div 100>.<N mod 100>`, where N is the number of commits on the first line of its history (the own line of `main`; a branch that is merged adds one commit, its merge). This command prints it:

```sh
sh deploy/bin/commit-version origin/main
```

On 2026-11-18 the first line of `main` had 800 commits, which gives 0.8.0; it is more than the published 0.2.1. No person writes a version, and no file holds one. A build by hand with `trial-bundle.sh` and no version gets the version of the commit that is checked out: on `dev` that is a number of the line of `dev`, which a commit of `main` can also have (R-202). Such a build does not update itself. The command refuses a shallow clone.

**The condition (R-201).** `main` must move only by a commit whose first parent is the tip of `main` before it: a merge of `dev` with `--no-ff`, or a commit directly on `main`. A fast-forward of `main` to a branch can make the count fall or jump. The first job of a run fails, with no release, when the tip of `main` before the push is not on the first line of the pushed commit.

### The branch `dev` and the act that releases (R-198)

The daily work goes to the branch `dev`. Agents start from `origin/dev`, merge into `dev` and push their claims to `dev`; they do not push to `main` (AGENTS.md). `main` holds what is released.

`dev` exists since 2026-11-19: the orchestrating session made it at `eb226c8d`, the tip of `main` then, on the instruction of the operator.

To release what `dev` holds: a merge commit on `main`, never a fast-forward.

```sh
git fetch origin
git switch --detach origin/main
git merge --no-ff origin/dev -m "Release: dev into main"
git push origin HEAD:refs/heads/main
```

The push is the release, when the switch is on. `dev` needs no merge back: the next merge of `dev` into `main` still works.

The branch rule set `main line` keeps the first line of `main`: it restricts deletions and blocks force pushes ("The state of GitHub"). It must not get "Require linear history": that forbids the merge commit that the version needs. The rule set `release tags` is no longer used by the workflow; it can stay.

What each other stream that pushes to `main` today must be told (Codex, and each coding agent with rules of its own): (1) start from `origin/dev` and merge into `dev`; (2) never push to `main`: a push to `main` is a public release; (3) never fast-forward `main`; (4) the base of a gates run does not change (`BASE_REF`, default `phase-2`).

### The state of GitHub

Read from GitHub on 2026-11-19 with `gh`. The operator, or the orchestrating session on his instruction (R-195), set each item. Never put a key or a password in a chat, a commit or a log.

| Item | State |
| --- | --- |
| Who can write | One account, the operator's |
| Workflows of outside contributors | Approval required for all external contributors |
| Workflow permissions | Read only |
| Tag rule set `release tags` | Active. Pattern `v*`. Creations, updates and deletions are restricted; force pushes are blocked. Only a repository administrator can bypass it |
| Branch rule set `main line` | Active since 2026-11-19 (id 27625577). Target `main`. Deletions are restricted and force pushes are blocked. No bypass. The orchestrating session made it on the instruction of the operator |
| Branch `dev` | Exists since 2026-11-19, made at `eb226c8d` |
| Variable `ANIMA_PUBLIC_RELEASES` | Does not exist: public releases are off |
| Environment `release` | Permits the branch `main` and tags `v*`. No required reviewer. Administrators can bypass |
| Environment `release-sign` | Permits the branch `main` and tags `v*`. No required reviewer. Administrators can bypass |
| Environment `release-publish` | Permits tags `v*` only (rule id 67371066); the jobs `bucket` and `publish` of a run of `main` are refused until the branch `main` is added ("To switch public releases on"). No required reviewer. Administrators can bypass |

The secrets and variables. GitHub lists names and times, never values. A run must prove that each value is usable.

| Environment | Use | Names |
| --- | --- | --- |
| `release` | macOS Developer ID signing and notarization (job `macos`) | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` |
| `release` | Not read by the workflow | `APPLE_SIGNING_IDENTITY` |
| `release` | iOS app signing and App Store Connect upload; the iOS job is not built | `IOS_DISTRIBUTION_P12_BASE64`, `IOS_DISTRIBUTION_P12_PASSWORD`, `IOS_PROVISIONING_PROFILE_BASE64`, `ASC_API_PRIVATE_KEY`, `ASC_API_KEY_ID`, `ASC_API_ISSUER_ID` |
| `release-sign` | The updater key (job `sign`) | `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |
| `release-publish` | The token of the bucket `anima-releases` (jobs `bucket` and `publish`) | `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, and the variable `RELEASES_ENDPOINT` |
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

1. Actions > release > "Run workflow". Branch: `main`. Version: `0.0.1`. Gates: `reuse` does what a release does (the Linux gates do not run when this content passed them on `dev`); `run` runs them in each case, to measure them or to prove the call. Do not give the version of a release: the trial makes a real signed and notarized application and a real updater signature for the version that you give, and a person who is signed in to GitHub can download the files of a run of a public repository for 7 days.
2. The run asks for no approval. It passes when the job `trial` passes. The job `publish` is skipped. Nothing is in the bucket and no feed changed. The application of a trial run is the application that a release makes: it has the updater. So when you start it, it reads the public feed and offers the published version, which is higher than `0.0.1`; that is correct, and it shows that the updater is on. Settings > Updates must not say "This is a local build".
3. From the page of the run, download the artifacts `macos` and `linux`. On a Mac:

   ```sh
   xcrun stapler validate Anima_0.0.1_universal.dmg
   spctl --assess --type open --context context:primary-signature --verbose Anima_0.0.1_universal.dmg
   ```

   Open the DMG, then `spctl --assess --type execute --verbose "/Volumes/Anima.ing/Anima.ing.app"` (expected: `accepted`, `source=Notarized Developer ID`). On a Linux computer with `x86_64`: `chmod +x Anima_0.0.1_amd64.AppImage` and start it. The operator says that an older AppImage of Anima ran on Linux; this check is for the AppImage of today, and the first public release waits for it (R-192).

### To switch public releases on

Do this only when the trial has passed and the AppImage of the trial has started on a Linux computer (R-192). The trial passed on 2026-11-19 (run 44479803653, HT-036). Two steps. The operator does them, or the orchestrating session on his instruction (R-195); each was checked against `.github/workflows/release.yml` on 2026-11-19.

1. Let the environment `release-publish` take a run of `main`. Today it permits only tags `v*` (one rule, id 67371066), and the jobs `bucket` and `publish` of a run of `main` name that environment: GitHub refuses them until `main` is added. The rule `v*` can stay; no tag starts the workflow.

   ```sh
   gh api --method POST repos/TheArctesian/anima.ing/environments/release-publish/deployment-branch-policies -f name=main -f type=branch
   ```

   To check: `gh api repos/TheArctesian/anima.ing/environments/release-publish/deployment-branch-policies --jq '[.branch_policies[]|{name,type}]'` gives `main` (branch) and `v*` (tag). The environments `release` and `release-sign` already permit `main`.
2. Set the switch. The first job of a run and the jobs `bucket` and `publish` read it:

   ```sh
   gh variable set ANIMA_PUBLIC_RELEASES --body on --repo TheArctesian/anima.ing
   ```

To switch them off again: `gh variable delete ANIMA_PUBLIC_RELEASES --repo TheArctesian/anima.ing`. To stop a run that already started, cancel it on GitHub.

### The first public release

It carries macOS and Linux. Its version is the version of its commit (R-183, R-201): on 2026-11-19 `main` was at 0.8.14, so the first release is 0.8.15 or more.

#### What the first push with the switch on does

This was read on 2026-11-19 from the three feeds and from `deploy/bin/release-decision` for the tip of `main`; nothing was published for it.

| Step | What happens |
| --- | --- |
| The decision | The feeds give 0.2.1 for `darwin-aarch64` and `darwin-x86_64`, and `linux-x86_64` answers 404. A feed that does not exist releases, so the first push to `main` after the switch is a release, also when it changes only a document. The decision for the tip of that day was `release 0.8.14` |
| The first minute | The job `bucket` reads `darwin-aarch64/latest.json` from the bucket with the token. A wrong token, endpoint or environment rule stops the run here, before the builds end |
| About 60 to 70 minutes | `gates-macos`, `tool`, `macos` and `linux` at the same time, and `gates-linux` when this content did not pass the gates on `dev`; then `gates` and `sign`. The macOS build (62 minutes in the second trial) is now the slowest part |
| `publish`, Linux first | `linux-x86_64/<version>/Anima_<version>_amd64.AppImage`, `…/SHA256SUMS` and `…/Anima_<version>_amd64.AppImage.sig`; the copy `Anima-x86_64.AppImage` at the root; then the new feed `linux-x86_64/latest.json` |
| `publish`, macOS second | `darwin-universal/<version>/Anima.app.tar.gz`, `…/Anima_<version>_universal.dmg`, `…/SHA256SUMS` and `…/Anima.app.tar.gz.sig`; the copy `Anima.dmg` at the root, which replaces the DMG of 0.2.1; then `darwin-aarch64/latest.json` and `darwin-x86_64/latest.json`, which replace the feeds of 0.2.1 |
| The end | The job reads the three feeds at `https://releases.anima.ing`; each must give the version |

Each file is read back from the bucket and compared, and each feed is written last. The files of 0.2.1 under `darwin-universal/0.2.1/` stay.

A Mac with 0.2.1 from the DMG: the application reads its feed at start and each 4 hours (UPD-001). It then shows the update to the new version in Settings > Updates and in the banner, and "Restart to update" installs it. It trusts the update: the public key in `tauri.conf.json` is the same text as at the commit of 0.2.1 (`b361eb5f`), and the trial proved that the key of `release-sign` signs for that public key. 0.2.1 carries the command line under its old name `anima-agent`; the new version carries `anima`. An application that a computer built does not update itself (DEF-001).

#### What can fail at the first real release that a trial does not show

| What | Why a trial does not show it | What is done, and what to do |
| --- | --- | --- |
| The environment `release-publish` refuses a run of `main` | A trial never uses that environment | Step 1 of "To switch public releases on". If it is not done, the job `bucket` fails in the first minute |
| The bucket token is wrong, has no right for `anima-releases`, or is of a different account than `RELEASES_ENDPOINT` | A trial has no token. The orchestrating session set the two secrets from a file of the operator (R-195); no run used them | The job `bucket` reads one object with them in the first minute. The variable has the right form (`https://<32 hex characters>.r2.cloudflarestorage.com`, read on 2026-11-19) |
| The token can read but cannot write | The job `bucket` only reads | The first write is a file of Linux under a new version folder; the feeds are not changed when it fails. Make a token with "Object Read & Write" for that bucket, set the two secrets, and select "Re-run failed jobs" |
| The public address does not give a new file | No trial puts a file | `publish` reads each file at `https://releases.anima.ing` before it writes a feed. The address did not keep an answer in its cache on 2026-11-19 (`cf-cache-status: DYNAMIC` for a feed, for the DMG and for the 404 of Linux) |
| `publish` stops between Linux and macOS | — | The Linux feed is new and the macOS feeds still give 0.2.1. "Re-run failed jobs" writes the same files again and goes on. If you do not, the next push that is a release publishes both |
| The AppImage does not start on a Linux computer | No machine of GitHub starts it | The check of the operator (R-192, HT-036 step 2). It is the reason that the switch is off |

#### The steps

1. Keep the three feeds of today, for a rollback:

   ```sh
   for feed in darwin-aarch64 darwin-x86_64 linux-x86_64; do curl -fsS -o "feed-$feed.json" "https://releases.anima.ing/$feed/latest.json" || echo "$feed has no feed"; done
   ```
2. Switch public releases on. The switch alone releases nothing: a run starts only from a push.
3. Push to `main` (the merge of `dev`, above).
4. Check, after the job `publish` passed:
   - Each feed gives the version (the job also reads them): `curl -fsS https://releases.anima.ing/darwin-aarch64/latest.json`, and the same for `darwin-x86_64` and `linux-x86_64`.
   - Download `https://releases.anima.ing/Anima.dmg` and run the two commands of the trial on it and on the application in it.
   - On a Mac with 0.2.1 installed from its DMG: Settings > Updates offers the version, and "Restart to update" starts it.
5. Set the release record at the top of this document, and the result of HT-036. The text for the two rows, with the version, the day and the commit of the run:

   - macOS, "Newest version": `<version> (<day>, commit <commit>), the first release of the release workflow. 0.2.1 was the last release of the Builds path (2026-11-17, commit b361eb5f)`. "Evidence": `Run <number> of the workflow release; the three feeds answered 200 with <version> on <day> (curl)`.
   - Linux, "Newest version": `<version> (<day>, commit <commit>), the first Linux release`. "State": `Published`. "Evidence": the same run.

#### The notes of the first release

For the page of the download and for a message to the persons who have 0.2.1. They describe `main` at 0.8.14 (`7c8ece8d`); read them again against the commit that is released.

- **Linux.** The first release for Linux: an AppImage for `x86_64`.
- **Sign in with an Anima.ing account.** The first run is "Sign in": the application signs in to the Anima.ing account and joins a hosted hub with no other step. The computer that signs in is a device of its hub.
- **Containers.** The page of a device has a tab Containers: the containers of the device, their details and logs, the state of a Compose project, images, volumes and networks, and a mark when a newer image exists.
- **Agent usage.** A page with the use of coding agents (Claude Code, Codex) for each device, model and day, with an estimate of the cost.
- **Logs.** The event log has a search by text.
- **When the hub does not answer.** The application shows a read-only copy of the last answers of the hub, with its age.
- **The command line.** The program is `anima` (it was `anima-agent` in 0.2.1). New commands: `anima containers`, `anima agents usage`, `anima audit verify`, `anima location`, `anima reminder`, each with its MCP tool.
- **Versions.** The version is now the number of the release commit, for example 0.8.15. The numbers are higher than 0.2.1 and have gaps; each is one commit of `main`.
- **Updates.** From this version, the application gets each change of `main` that changes it, with no step of a person. Settings > Updates shows the version and the check.

### `anima` alone: Homebrew and crates.io (R-217 to R-219)

Built, and never run with a switch on. A release of the desktop application can also release `anima` alone, in three jobs after the job `publish`:

| Job | What it makes public | Secret, environment |
| --- | --- | --- |
| `cli-publish` | `anima/<version>/anima-darwin-universal.tar.gz`, `anima/<version>/anima-linux-x86_64.tar.gz`, `anima/<version>/SHA256SUMS`, then `anima/latest.json`, at `https://releases.anima.ing` | The bucket token, `release-publish` |
| `tap` | `Formula/anima.rb` in `TheArctesian/homebrew-anima`: `brew install thearctesian/anima/anima` | `HOMEBREW_TAP_DEPLOY_KEY`, `release-tap` |
| `crate` | The crate `anima-cli` on crates.io, with the version of the release | `CARGO_REGISTRY_TOKEN`, `release-crate` |

The job `cli`, with no secret, makes the files first: each `anima` is the program inside the application that the same run built (the macOS one has its Developer ID signature), and the job starts the Linux program to see that it runs alone. A trial runs `cli` and none of the three jobs above.

**The switch of its own.** The three jobs run only while `ANIMA_PUBLIC_RELEASES` and a second variable, `ANIMA_PUBLIC_CLI`, are both `on`. The second does not exist today. So the first release of the desktop application does not depend on these jobs, and the desktop jobs never wait for them.

**A version on crates.io is permanent (R-219).** It cannot be deleted, only marked as withdrawn (`cargo yank`). With the switch on, each push to `main` that changes `crates/anima-cli`, the lock file or the release files becomes a version there for good. The name `anima-cli` was free on 2026-11-19; the first publish takes it for the account of the token.

Before `ANIMA_PUBLIC_CLI` is set, the operator does these steps. No agent does them (R-180, R-195): each makes a credential.

1. **The key of the tap.** A deploy key can write to one repository only. On your Mac:

   ```sh
   ssh-keygen -t ed25519 -N "" -C "anima release workflow" -f ~/.config/anima/tap-key
   gh repo deploy-key add ~/.config/anima/tap-key.pub --repo TheArctesian/homebrew-anima --allow-write --title "anima release workflow"
   ```
2. **The environment `release-tap`**, which permits only `main`, with the key as its secret:

   ```sh
   gh api --method PUT repos/TheArctesian/anima.ing/environments/release-tap --input - <<'JSON'
   {"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}
   JSON
   gh api --method POST repos/TheArctesian/anima.ing/environments/release-tap/deployment-branch-policies -f name=main -f type=branch
   gh secret set HOMEBREW_TAP_DEPLOY_KEY --env release-tap --repo TheArctesian/anima.ing < ~/.config/anima/tap-key
   rm ~/.config/anima/tap-key
   ```
3. **The token of crates.io.** Sign in at crates.io with your GitHub account and confirm your email address there (crates.io refuses a publish without it). Account Settings > API Tokens > New Token: name `anima release workflow`; scopes "publish-new" and "publish-update"; crates `anima-cli`; an end date that you select. It shows the token one time.
4. **The environment `release-crate`**, the same way, then the token. The command asks for the value and does not show it:

   ```sh
   gh api --method PUT repos/TheArctesian/anima.ing/environments/release-crate --input - <<'JSON'
   {"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}
   JSON
   gh api --method POST repos/TheArctesian/anima.ing/environments/release-crate/deployment-branch-policies -f name=main -f type=branch
   gh secret set CARGO_REGISTRY_TOKEN --env release-crate --repo TheArctesian/anima.ing
   ```

   After the first publish, a token for only "publish-update" of `anima-cli` is enough: make it, set the secret again, and revoke the first.
5. **The switch**, only after a release of the desktop application is public and a trial has passed with the job `cli`:

   ```sh
   gh variable set ANIMA_PUBLIC_CLI --body on --repo TheArctesian/anima.ing
   ```

If a job names an environment that does not exist, GitHub makes it with no rule; so make the two environments before the switch. To stop: `gh variable delete ANIMA_PUBLIC_CLI --repo TheArctesian/anima.ing`. A version that is on crates.io stays; `cargo yank --version <version> anima-cli` marks it as withdrawn.

What is not known until the first run: if the Linux program starts on a computer with no `libdbus-1` (it uses the key store of the desktop through that library), if R2 takes the `put-object` of the `aws` program of the machine, and what `brew audit` says of the formula in the tap. HT-041 has the checks.

### The iOS application: TestFlight (R-203; task B8i)

Built as three jobs of the release workflow, and never run: `ios` builds and signs, `ios-upload` sends the file to App Store Connect, and `ios-record` writes `ios/latest.json` in the bucket so that the next push can compare with it. The job `ios` calls `apps/desktop/src-tauri/ios-archive.sh`, which is task M5-4 of the [mobile plan](../plans/mobile-plan.md) and does not exist yet.

**Two switches of its own.** `ANIMA_IOS_BUILD` lets the job `ios` run, in a trial and for a push to `main`: it builds and signs, and uploads nothing. While it is not `on`, no iOS job runs, and a trial is not red for it. `ANIMA_PUBLIC_TESTFLIGHT`, which only the operator sets, lets a push to `main` also send that build to TestFlight. Neither exists today, and they do not depend on `ANIMA_PUBLIC_RELEASES`.

If a build goes to the operator alone before the sign-in of the phone works is not decided. Asked, he said (2026-11-19, relayed): "sign in does kinda work rn but i get that there are some bugs. Sign in on I phone should also be native tho". So the workflow makes both possible and decides neither: with the TestFlight switch off nothing leaves GitHub. With it on, the upload gives the build to App Store Connect, where it is for internal testers only: the workflow never adds a build to a group of outside testers, and never sends one to the review of Apple. The first public TestFlight build waits for R-158; to give a build to outside testers is an act of the operator in App Store Connect.

**The build number** is the count of the first line of `main`: version 0.8.15 has the build number 815 (R-201). It only goes up on `main`, as App Store Connect asks. The archive script sets it from the version (`deploy/bin/phone-build-number`, task M5-3 of the mobile plan).

**The script.** The job calls `sh apps/desktop/src-tauri/ios-archive.sh <version>` with `IOS_SIGN_IDENTITY` (the name of the Apple Distribution identity in the keychain of the job) and `IOS_PROFILE` (the path of the profile file). The script gets no certificate, no password and no key of App Store Connect, and it uploads nothing. Its last lines say `version=`, `build=`, `signed=` and `ipa=`; the job takes the file only when they say the version of the release and `signed=yes`. This interface is the proposal of the mobile session of 2026-11-19, on its branch `mobile/account-sign-in`; that session says that its signed mode is not tested.

Before `ANIMA_IOS_BUILD` is set, the operator does these steps. No agent does them (R-180, R-195). The six iOS secrets are in the environment `release` today (R-204); GitHub cannot copy a secret, so each is set again from its file. Until then the job `ios` fails with the words "The three iOS signing secrets are not set in the environment release-ios"; it does not read them from `release`.

1. **In App Store Connect:** an app record with the bundle ID `ing.anima.mobile` (My Apps > "+" > New App); in its TestFlight tab, one internal group (Internal Testing > "+") with only your own account as tester, and no external group; an API key (Users and Access > Integrations > App Store Connect API) with the role "App Manager" or "Developer". The Apple Distribution certificate and the App Store profile of `ing.anima.mobile` exist already (the secrets of 2026-11-19).
2. **The environment `release-ios`** (the certificate and the profile; the job that builds), for `main` only:

   ```sh
   gh api --method PUT repos/TheArctesian/anima.ing/environments/release-ios --input - <<'JSON'
   {"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}
   JSON
   gh api --method POST repos/TheArctesian/anima.ing/environments/release-ios/deployment-branch-policies -f name=main -f type=branch
   base64 -i <the .p12 file> | tr -d '\n' | gh secret set IOS_DISTRIBUTION_P12_BASE64 --env release-ios --repo TheArctesian/anima.ing
   gh secret set IOS_DISTRIBUTION_P12_PASSWORD --env release-ios --repo TheArctesian/anima.ing
   base64 -i <the .mobileprovision file> | tr -d '\n' | gh secret set IOS_PROVISIONING_PROFILE_BASE64 --env release-ios --repo TheArctesian/anima.ing
   ```

   The second command asks for the password and does not show it.
3. **The environment `release-testflight`** (the key of App Store Connect; the job that uploads), the same way:

   ```sh
   gh api --method PUT repos/TheArctesian/anima.ing/environments/release-testflight --input - <<'JSON'
   {"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}
   JSON
   gh api --method POST repos/TheArctesian/anima.ing/environments/release-testflight/deployment-branch-policies -f name=main -f type=branch
   gh secret set ASC_API_PRIVATE_KEY --env release-testflight --repo TheArctesian/anima.ing < <the AuthKey_….p8 file>
   gh secret set ASC_API_KEY_ID --env release-testflight --repo TheArctesian/anima.ing
   gh secret set ASC_API_ISSUER_ID --env release-testflight --repo TheArctesian/anima.ing
   ```
4. **After a run has used them:** delete the six iOS secrets from `release`, where the macOS build has them in its memory: `gh secret delete <name> --env release --repo TheArctesian/anima.ing` for each. R-204 left them there; this step is yours to take or not.
5. **The switches**, the first when the archive script is on `main`, the second when a trial has built and signed:

   ```sh
   gh variable set ANIMA_IOS_BUILD --body on --repo TheArctesian/anima.ing
   gh variable set ANIMA_PUBLIC_TESTFLIGHT --body on --repo TheArctesian/anima.ing
   ```

Two environments and not one: the job that builds runs the code of the commit, and the key that can upload to your App Store Connect account does not need to be there.

Not proven, each until a real run: that the phone shell builds on a machine of GitHub; that the script signs from a keychain and an installed profile, with no window; that `xcrun altool --upload-app` of the machine takes the file with the API key; the time of the job; what App Store Connect answers for the first build of a new app (it can ask for export compliance and for a review of the beta). HT-046 has the checks.

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
[s12-packaging-tasks.md](../plans/one-click/s12-packaging-tasks.md)). Since task HM-T16 it also
builds `anima-daemon`, the daemon that the application installs on the computer it runs on
(HMD-052 of [hosted-mode.md](../proposals/hosted-mode.md), R-206):

| File | macOS | Linux AppImage |
| --- | --- | --- |
| `anima`, `anima-local-hub`, `anima-hub`, `anima-database`, `anima-gateway`, `anima-daemon`, `anima-link` | `Anima.ing.app/Contents/MacOS/` | `usr/bin/` |
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
