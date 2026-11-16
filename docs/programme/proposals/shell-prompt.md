# Proposal: Anima.ing shell prompt

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-16. The operator asked for "anima as your prompt", then
said that Starship is a perfect example, and pointed at the fish configuration in the
DotFiles and `.config` repositories (oh-my-fish theme boxfish, Zellij, and a few helper
functions).
Version: 1. Date: 2026-11-16.
Status: Draft. The requirements are not yet approved. No ruling.
Map: [work-plan.md](../../work-plan.md). The command line is not a category (Section 3). This
proposal extends the `anima` program (CMD) and touches installation (INS-009). If the operator
approves it, the new command identifiers move into [cli-mcp-plan.md](../../cli-mcp-plan.md)
(PLN-004).
Identifiers: CMD-032 to CMD-036.

## Problem

The `anima` command line is on `main` and already does the work of the hub. The shell prompt
around it is still the operator's ordinary bash, zsh or fish prompt. There is no branded
line that shows the hub and the device, and no one-click way to turn that line on.

A new shell of our own would fight fish and zsh, would break login shells, and cannot include
fish: fish is GPL-2.0, and this command line is AGPL.

## Recommendation

A Starship-style prompt that sits on top of the existing bash, zsh or fish. Ordinary commands
still go to the operator's shell. `anima …` stays the CLI that is already on `main`. The
reference is [Starship](https://starship.rs) (ISC): we can borrow its approach, and even its
code if we keep its notice. The look of the default theme is
[boxfish](https://github.com/joelwanner/theme-boxfish) (MIT), the oh-my-fish theme in the
operator's fish configuration: compact boxed segments in plain ASCII, not powerline arrows.
Arrow-style themes can come later as alternative theme files.

The first useful moment is a clean terminal and one click in Settings. The prompt redraws
with the logo and the hub, an ordinary command runs as usual, and `anima ssh studio` opens a
session on another machine, all in about 15 seconds.

## Commands

- **CMD-032.** `anima init zsh|bash|fish` must print a snippet that the operator can paste
  into the startup file of that shell. The snippet must be marked, so a later remove can take
  out only that block. The command must not edit any file.
- **CMD-033.** `anima prompt` must draw one powerline-free line of boxed segments and then
  exit. It must draw in about 20 ms and must never wait on the hub. If the session is not
  interactive, it must print nothing, so `scp`, `rsync` and scripts do not break.
- **CMD-034.** `anima layout claude` and `anima layout codex` must open a Zellij layout with
  yazi next to Claude or Codex, on the model of the operator's `zcc` and `zco` functions.
  They must not replace Zellij. Agent panes (AIS-009 to AIS-017) must work alongside Zellij
  in the first version, not replace it.
- **CMD-035.** `anima files pack <dir>` must bundle a folder and leave out `node_modules` and
  `.git`, on the model of the operator's `copydir` function.
- **CMD-036.** `anima greeting` is optional. It may replace a tool such as pfetch with a
  short status of the hub, the devices and any agents that wait for the operator.

A wrapper REPL called `anima shell` is later work: autosuggestions for `anima` commands, and
never the login shell.

## Default theme

The default theme is a boxfish-style line of colour-coded boxes, in Graphite or Nord (Nord
is already the palette of the operator's Zellij setup). The boxes come in this order:

1. The Anima.ing logo. The first version uses a plain `A.` box. A Nerd Font glyph is an
   upgrade when the font can show it. If the font cannot show the glyph, the prompt falls
   back to `A.`
2. A red `!` when the last command failed.
3. The shortened path of the current folder.
4. The git branch: yellow when there are uncommitted changes, green when the tree is clean.
5. The hub that this shell uses, from a local cache, never from a live hub request.
6. A short device hint.

Git status is in the first version. Theme files are plain data that the operator can edit.
They are never scripts.

## Settings

Settings gets an opt-in switch, "Use the Anima.ing prompt". The switch must ask first, write
one marked block to the startup file, and do nothing if the operator cancels. Remove must
take out only that block.

This needs a ruling on INS-009. INS-009 says that the installer never edits the shell
profile: it only shows the `PATH` line for the operator to paste. Pasting the snippet from
`anima init` works either way. The Settings switch is the part that writes the file.

## Constraints

- **INS-009.** The app must not edit `.bashrc`, `.zshrc` or the fish startup file unless the
  operator confirms a marked edit, and only after a ruling that allows that exception.
- **K4 exec.** Smart routing such as `@studio git status` is not possible yet. The hub's
  tunnel contract opens a login shell and sends no `exec` (`contracts/tunnel.md`). For now,
  routing means `anima ssh studio`.
- **EXT-009.** Shared community themes wait until the hub is allowed to run other people's
  code. Until then, a theme is local data on the operator's machine.
- **No new shell.** Do not write a full shell. Fish is GPL-2.0, so we must not build it into
  the AGPL command line.

## Open questions

1. Which startup file to use when the operator runs oh-my-zsh, or fish with `conf.d`?
2. Should the prompt read its theme from the desktop app or from its own small config?
3. Should the logo stay a plain `A.` box, use an existing Nerd Font glyph, or add a custom
   glyph? This draft uses `A.` with a Nerd Font upgrade.
4. Git status in the first version: settled in this draft, yes, because boxfish already
   shows the branch and the dirty state.

## Needs

- A ruling on INS-009 for the Settings switch.
- After approval: add CMD-032 to CMD-036 to [cli-mcp-plan.md](../../cli-mcp-plan.md), and a
  test that `anima prompt` stays under the time limit and prints nothing in a non-interactive
  shell.
