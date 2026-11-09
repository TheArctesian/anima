# guacd fixtures for RDP and VNC

Recorded from the `guacamole/guacd:1.6.0` image (guacamole-server 1.6.0). Each `.guac` file holds
instructions byte for byte as guacd sent them, with no trailing newline. The tests load them with
`include_bytes!` and decode them with `guac-protocol`'s `Decoder`. Do not edit them; add new files
for a new guacd release.

## `args` for a new connection

Each file holds the one `args` instruction that guacd sent after `select,rdp` or `select,vnc`.

| File | guacamole-server | Names after the version |
| --- | --- | --- |
| `guacd-1.6.0-rdp-args.guac` | 1.6.0 | 88 |
| `guacd-1.6.0-vnc-args.guac` | 1.6.0 | 51 |

## Joining a VNC connection (K4 Sections 3 and 10, GW-014, RA-035)

| File | What guacd sent |
| --- | --- |
| `guacd-1.6.0-vnc-ready.guac` | `ready` to the owner after `select,vnc` and `connect`: the connection id, `$` and a lowercase UUID (37 characters) |
| `guacd-1.6.0-vnc-join-args.guac` | `args` after `select,$<that id>`: byte for byte the same as `guacd-1.6.0-vnc-args.guac` |
| `guacd-1.6.0-vnc-join-opening.guac` | To a joiner with `read-only` `true`: `ready` with the owner's id, then the owner's screen (cursor layer, `size` of layer 0 at the remote's 64 x 48, `img` streams, `set`, `cursor`, `mouse`) up to the first `sync` |
| `guacd-1.6.0-vnc-join-owner-left.guac` | To that joiner after the owner sent `disconnect` and before the joiner sent its own: only frames (`rect`, `cfill`, `copy`, `sync`) |
| `guacd-1.6.0-join-unknown.guac` | `error` `No such connection.` `516` (0x0204) after `select` with an unknown `$` id; guacd then closed the socket |
| `guacd-1.6.0-vnc-join-observed.json` | What the fake VNC server and the recording client saw in the same run (see below) |

The connection id in these files belongs to a container that no longer exists.

### How they were recorded

guacd ran as `docker run -d --name wp47-guacd -p 127.0.0.1:4822:4822 guacamole/guacd:1.6.0`
(image digest `sha256:8974eaa9ba32f713daf311e7cc8cd7e4cdfba1edea39eed75524e78ef4b08f4f`), with its
port on loopback only. The VNC server was a Python standard-library RFB 3.8 server on the docker
bridge address `172.17.0.1`, port 5909: security type None, no password, a 64 x 48 framebuffer, raw
updates, and a log of every client message with its decoded fields. A Python standard-library client
spoke the Guacamole protocol to guacd, wrote every received instruction to a file, and answered each
`sync` with `sync`, as guacamole-common-js does. The only `connect` values were `hostname`
`172.17.0.1` and `port` `5909` for the owner, and `read-only` for the joiners. The scripts are not
in the repository; WP-47's package file records the method.

In one run, in this order:

1. The owner: `select,vnc`, `size` 1024 x 768 x 96, `audio` and `video` empty, `image` PNG, JPEG and
   WebP, `timezone` `UTC`, `name`, `connect`, then `ready`.
2. A joiner: `select,$<id>`, `size` 800 x 600 x 96, the same `audio`, `video`, `image` and
   `timezone`, `name` `viewer`, and `connect` with `VERSION_1_5_0`, `read-only` `true` and every other
   value empty. guacd answered `ready` with the owner's id.
3. Input, each sender with its own pointer position and keysym so the VNC log shows who reached the
   server: the read-only joiner at (10,10) with keysym 97, the owner at (20,20) with 98, a second
   joiner with `read-only` empty at (30,30) with 99, and a third with `read-only` `false` at (40,40)
   with 100. Each sent `mouse` with buttons 1 then 0, and `key` down then up.
4. `select` with `$00000000-0000-4000-8000-000000000000`.
5. The owner sent `disconnect` and closed its write side. Eight seconds later a new joiner with
   `read-only` `true` joined the same id. Then each joiner sent `disconnect`.

What the observation file records:

- `input`: the read-only joiner's `mouse` and `key` never reached the VNC server. The owner's, the
  empty joiner's and the `false` joiner's did. guacd opened one RFB connection (shared flag 1) for
  all four users.
- `unknown_id`: one instruction, then guacd closed the socket.
- `owner_leaves`: the owner got `disconnect` back. The joiners got no notice and kept getting
  frames. The new join after the owner left got `ready`. guacd closed the owner's socket only when
  the last joiner had gone. Each joiner's socket closed as soon as it sent `disconnect`.

guacd also sent the owner `msg` `1` `@<user id>` `<name>` for each joiner, and only to the owner.

In an earlier run the client did not answer `sync`. guacd then dropped each joiner 15 seconds after
that joiner's last instruction, with `error` `Aborted. See logs.` `776` and `disconnect`, and did not
close the socket in the next 60 seconds, while the client kept its own side open. The files above
come from the run that answered `sync`.
