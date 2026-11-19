---
kind: Proposal
state: Draft
evidence: Not approved. REP-001 to REP-025. MET-002, USE-006, INS-009 and VLT-007 ruled 2026-11-16.
---

# Proposal: machine profiles

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-16. The operator asked for a one-click way to replicate
a setup across Macs and other computers: the same apps, the same configuration, keys and
files, after the person has signed into iCloud and finished the operating system's first
setup. The operator thought a "whatdog" plan in the Marketing Bot might already cover this.
Version: 1. Date: 2026-11-16.
Status: the header of this document holds the state (PLN-020). History: Draft. The requirements are not yet approved. No ruling.
Map: [work-plan.md](../work-plan.md), category Foundation, next to "Add this machine"
([add-this-machine.md](add-this-machine.md)) and first-run setup. A rented cloud computer
([cloud-machines.md](cloud-machines.md)) can take a profile in one confirmation. A profile
can also install the agent tools (Herdr, Hermes Agent, T3 Code: [agent-tools-as-extensions.md](agent-tools-as-extensions.md), R-196) and the agents of [bots.md](bots.md).
Identifiers: REP-001 to REP-025.

This is a new proposal. The "whatdog" plan is Watchdog, a service restarter planned with the
Marketing Bot. Its proposal sits on the branch `claude/watchdog-autoupdate-proposals` and is
not on `main`. Nothing in that work, or on `main`, covers copying a whole setup.

## Problem

A new computer, after macOS setup and iCloud sign-in, still needs hours of work: install the
same apps, copy dotfiles, make or move keys, and bring over the folders that are not already
in iCloud. "Add this machine" enrols the computer. It does not make the computer like the
others.

## Prior art

| Product | What it copies | What this proposal takes | What it does not do for Anima.ing |
| --- | --- | --- | --- |
| Apple Migration Assistant and Setup Assistant | A whole user, including keychains and many permissions, usually over USB or a local network | The moment: after iCloud sign-in, offer the rest of the setup | It is all-or-nothing, it is Mac only, and it is not a named, repeatable profile |
| iCloud Drive and Desktop and Documents | Folders the operator already put in iCloud | Skip those folders by default, so Anima does not copy them twice | It does not install apps or copy SSH keys |
| Homebrew Bundle and `mas` | A Brewfile of formulae, casks and App Store apps | The install path for apps on a Mac, and on Linux when Homebrew is there | The operator still has to be signed into the App Store; it cannot transfer a paid licence |
| chezmoi, yadm, Mackup, a dotfiles repository | Dotfiles, with chezmoi also able to handle some secrets carefully | A ticked set of dotfiles and the Anima.ing prompt as ordinary files | These tools do not install App Store apps or copy large folders |
| Nix, nix-darwin, home-manager | A declarative system and user environment | Later, perhaps, as one way to describe a profile | Not the first version: the operator's Mac setup today is Homebrew, fish and Zellij |
| Ansible or a similar tool | Repeatable roles across machines | The idea of a dry-run, then apply, with a report | Too much ceremony for a one-click consumer path |
| MDM (Jamf and the like) | Apps and settings for a fleet, with supervision | Nothing in the first version | MDM needs organisation enrolment and is the wrong model for a personal hub |
| Carbon Copy Cloner or a disk image | A whole disk | Nothing in the first version | It copies too much, including secrets the operator did not tick |
| Omarchy / Omnixy | A scripted Linux desktop | Background only: a one-shot setup of a personal machine | Linux first, not Mac to Mac, and not a profile inside Anima |

## How it works

After macOS setup and iCloud sign-in, the operator installs Anima.ing, enrols the machine,
and picks "Make this Mac like Studio" (or another source machine). The client shows a
dry-run preview of what will be installed, copied or skipped, then waits for a confirmation.
It installs the ticked apps through Homebrew Bundle and `mas`, copies the ticked dotfiles
and prompt, makes a new SSH key and can add it to GitHub, and copies chosen folders straight
across the network. Folders already in iCloud are skipped by default. It keeps an undo log.
A step that fails does not stop the rest.

## Requirements

- **REP-001.** After the operator signs into iCloud (on a Mac), installs Anima.ing and enrols
  the machine, the client may offer "Make this Mac like the source machine".
- **REP-002.** A profile is a named list of apps, settings, dotfiles, keys and folders taken
  from one source machine.
- **REP-003.** Before any change, the client must show a dry-run preview of what will be
  installed, copied or skipped, and why. The operator confirms before it starts.
- **REP-004.** The operator ticks each item in the preview. Unticked items are skipped.
- **REP-005.** Apps are installed through Homebrew Bundle (a Brewfile) where Homebrew can
  install them.
- **REP-006.** App Store apps are installed through `mas`. The operator must already be
  signed into the App Store.
- **REP-007.** Selected dotfiles and the Anima.ing prompt, if ticked, are copied to the same
  paths on the new machine.
- **REP-008.** By default, no private key is copied. Devices that Anima.ing can reach are
  reached through Anima.ing (`anima ssh`, or plain `ssh` through a host entry that Anima.ing
  writes), so the new machine needs no key for them (REP-025). For tools outside Anima.ing,
  such as git over SSH, the function makes a new SSH key on the new machine and can add it to
  GitHub, with the operator's confirmation.
- **REP-009.** Copying an existing private key is off by default. It is opt-in, one key at a
  time. The key travels straight from the source machine to the new machine and must never be
  written to the hub (a stricter path than VLT-007).
- **REP-010.** Selected folders are copied straight across the network, machine to machine,
  not through the hub.
- **REP-011.** Folders that already live in iCloud are skipped by default. The preview must
  say so.
- **REP-012.** The function keeps an undo log of each step it took, so the operator can
  reverse a change.
- **REP-013.** A step that fails must not stop the rest. The preview and a final report must
  show each failure and the next step.
- **REP-014.** The preview and the report must say what cannot be copied: privacy permissions
  (Full Disk Access, Accessibility and the like), the login and iCloud keychains, App Store
  sign-in, and paid app licences.
- **REP-015.** The function must not try to grant those permissions, copy those keychains,
  sign into the App Store, or transfer a paid licence.
- **REP-016.** Machine profiles is a free extension on every plan, including self-host.
- **REP-017.** A rented cloud computer (CVM-001) can take a profile in one confirmation after
  it boots.
- **REP-018.** A profile can include the agent tools of agent-tools-as-extensions.md and the agents of bots.md (R-196 withdrew the native panes and bot runtime), so a new
  machine can run them after the profile is applied.
- **REP-019.** The first version is Mac to Mac.
- **REP-020.** Linux to Linux follows, when both machines have Homebrew or a documented
  equivalent.
- **REP-021.** Cross-OS copy and Windows come later and are not in the first version.
- **REP-022.** Applying a profile is a work type on the fixed list of USE-006, proposed name
  `apply_profile`. The hub cannot name an arbitrary command. The daemon applies only the
  ticked items of a stored profile.
- **REP-023.** The daemon may install and copy when it applies a profile. This is an exception
  to MET-002, granted 2026-11-16. Whether each apply asks first is the user's setting; by
  default only steps that hand over a secret ask.
- **REP-024.** The Settings switch that writes the prompt snippet, if approved, uses the same
  marked-block rule as [shell-prompt.md](shell-prompt.md) (INS-009).
- **REP-025.** The profile may include an `ssh` host entry for each device, so that plain
  `ssh studio` goes through Anima.ing in the same way as `anima ssh studio`. Writing that entry
  is a marked block in `~/.ssh/config`, with the same yes-first rule as the prompt (CMD-038).

## First version and later

| First version (MVP) | Later |
| --- | --- |
| Mac to Mac, after iCloud sign-in and enrolment | Linux to Linux (REP-020) |
| Dry-run preview, ticked items, undo log, continue on failure | Cross-OS and Windows (REP-021) |
| Homebrew Bundle, `mas`, ticked dotfiles and prompt | A Nix or home-manager backend |
| New SSH key, optional add to GitHub | More forges than GitHub |
| Opt-in copy of one existing private key, machine to machine | — |
| Chosen folders over the network; iCloud folders skipped | — |
| One-confirm apply on a rented cloud computer | — |
| Agent tools and agents (agent-tools-as-extensions.md, bots.md) | — |

## Conflicts: ruled (operator, 2026-11-16)

- **General rule.** Each safety setting here is the user's choice. The default is the
  equivalent of full auto, as in MCP-024 (R-120). A step that hands over a secret, such as copying a
  private key (REP-009), still asks first.
- **MET-002:** Exception granted: the daemon may install apps and copy files when it applies a
  profile (REP-023). By the general rule, the per-apply confirmation is on by default only for
  steps that hand over a secret; the user can turn it on for every step.
- **USE-006:** `apply_profile` joins the fixed list of work types (REP-022). The hub still
  cannot name an arbitrary command.
- **INS-009:** The installer never edits the shell profile. `anima shell setup` and the Settings
  CLI installer add the line only when the operator says yes (shell-prompt CMD-037, CMD-038;
  REP-024).
- **VLT-007:** The default is no key copy: devices are reached through Anima.ing (REP-008,
  REP-025). A copied key, for tools outside Anima.ing, goes machine to machine and never
  through the hub (REP-009).

## Risks

- A confirmed apply still changes a computer: packages, startup files and keys. The dry-run,
  the ticks, the undo log and the report are the controls. A security review of the work
  route must include this type.
- A partial failure can leave a machine half-applied. REP-013 keeps going, but the report
  must make the leftover work obvious, and the undo log must be able to reverse the steps
  that did run.
- Copying a private key, even opt-in, is easy to get wrong. The key must not be logged
  (LOG-007), must not sit on the hub, and must be removed from any temporary path when the
  copy ends.
- Homebrew and `mas` can ask for a password or for App Store sign-in. The daemon must not
  collect those. The operating system, or the operator in a session, supplies them.
- App Store licences and macOS privacy permissions cannot be copied. If the preview does not
  say this clearly, the operator will think the new Mac is finished when it is not.

## Open questions

1. Where does a profile live: only on the source machine, or also as a sealed copy on the
   hub (without private keys)?
2. What happens when the source machine is offline? Is a cached profile enough for apps and
   dotfiles, with folder copy and key copy delayed?
3. How does the function choose the default source when the operator has more than two Macs?
4. Should a second apply of the same profile be incremental, or always a fresh dry-run?
5. For Linux, is Homebrew the right first tool, or should the first Linux path wait for a
   documented package list per distribution?

## Recommendation

Make this a free extension called Machine profiles, Mac to Mac first, with the dry-run and
the "cannot copy" list in the first version. Do not start Windows or cross-OS work. Do not
put private keys on the hub. MET-002, USE-006, INS-009 and VLT-007 are ruled
(2026-11-16). Treat Watchdog as a separate proposal on its own branch.

## Needs

- MET-002, USE-006, INS-009 and VLT-007: ruled 2026-11-16.
- After approval: a row in the extensions plan for the work package, and a security review
  of key copy, undo and the new work type.
- No change to the Watchdog branch as part of this work.
