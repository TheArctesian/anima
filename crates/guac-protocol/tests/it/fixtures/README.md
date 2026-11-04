# guacd `args` fixtures

Recorded wire forms of the `args` instruction that guacd sends after `select,ssh`.
Each file is one instruction, byte for byte, with no trailing newline.

| File | guacamole-server | Names after the version |
| --- | --- | --- |
| `guacd-1.5.5-ssh-args.guac` | 1.5.5 | 39 |
| `guacd-1.6.0-ssh-args.guac` | 1.6.0 | 43 (adds `timeout`, `public-key`, `typescript-write-existing`, `recording-write-existing`) |

Built from `GUAC_SSH_CLIENT_ARGS` in `src/protocols/ssh/settings.c` of each release, with the
version element `VERSION_1_5_0` (`GUACAMOLE_PROTOCOL_VERSION`), which both releases send.
The tests load them with `include_bytes!` and decode them with the crate's `Decoder`.
Do not edit them; add a new file for a new guacd release.
