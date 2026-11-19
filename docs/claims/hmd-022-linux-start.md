---
task: HMD-022
plan: docs/proposals/hosted-mode.md
machine: pop-os
agent: Codex
branch: fix/linux-start-sign-in
started: 2026-11-19
---

Correct the desktop start with no chosen hub: show the account sign-in screen even when an
older installation has a token for the default hub or the OS key store refuses a read.
