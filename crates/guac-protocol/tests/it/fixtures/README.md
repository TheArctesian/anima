# guacd `args` fixtures

Wire forms of the `args` instruction that guacd sends after `select,ssh`.
Each file is one instruction, byte for byte, with no trailing newline.

They are built from `GUAC_SSH_CLIENT_ARGS` in `src/protocols/ssh/settings.c` at tag 1.5.5 / 1.6.0,
as compiled in the official guacamole/guacd image (no `--enable-ssh-agent`); the `-agent-`
variants are with `--enable-ssh-agent`, which adds `enable-agent` after `public-key` (1.6.0) or
`passphrase` (1.5.5). The version element is `VERSION_1_5_0` (`GUACAMOLE_PROTOCOL_VERSION`),
which both releases send.

| File | guacamole-server | Names after the version |
| --- | --- | --- |
| `guacd-1.5.5-ssh-args.guac` | 1.5.5 | 38 |
| `guacd-1.6.0-ssh-args.guac` | 1.6.0 | 42 (adds `timeout`, `public-key`, `typescript-write-existing`, `recording-write-existing`) |
| `guacd-1.5.5-ssh-agent-args.guac` | 1.5.5, `--enable-ssh-agent` | 39 |
| `guacd-1.6.0-ssh-agent-args.guac` | 1.6.0, `--enable-ssh-agent` | 43 |

The tests load them with `include_bytes!` and decode them with the crate's `Decoder`.
Do not edit them; add a new file for a new guacd release.
