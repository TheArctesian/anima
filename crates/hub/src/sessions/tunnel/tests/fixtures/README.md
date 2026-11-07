# guacd `args` fixtures for RDP and VNC

Each file holds the `args` instruction that guacd sent after `select,rdp` or `select,vnc`. It is
one instruction, byte for byte as received, with no trailing newline. They were recorded from
the `guacamole/guacd:1.6.0` image.

| File | guacamole-server | Names after the version |
| --- | --- | --- |
| `guacd-1.6.0-rdp-args.guac` | 1.6.0 | 88 |
| `guacd-1.6.0-vnc-args.guac` | 1.6.0 | 51 |

The tests load them with `include_bytes!` and decode them with `guac-protocol`'s `Decoder`.
Do not edit them; add a new file for a new guacd release.
