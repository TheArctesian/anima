# The record of the mockup terminal-ui/index.html

One row of the index in [README.md](../README.md) (PLN-019). `bun run quality:register` writes the row from this file.

- Mockup: [terminal-ui/index.html](../terminal-ui/index.html)
- Owner: [cli-mcp-plan.md](../../plans/cli-mcp-plan.md) Section 11.4 (TUI-010 to TUI-034; task TU5)
- Screens: None yet. `anima tui` is not built: the code goes in `crates/anima-cli/src/tui/` (task TU2).
- State: Proposed
- Compared: Not compared
- Differences: None known. The mockup is drawn before the program. It is a terminal, so each frame is text at 80 columns in the Nord terminal tokens (`design/tokens/terminal.tokens.json`). Compare it in a real terminal at 80 by 24 and at 100 by 30 when TU5 is done, and with `NO_COLOR` set. The detail pane of frame 1 hides under 100 columns in the program (TU5).
