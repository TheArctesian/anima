# Native gateway (`anima-gateway`)

`anima-gateway` is a gateway that connects to VNC and RDP devices itself. It speaks to the hub as
`guacd` does (spec Section 4.3, K4 Section 3), so the hub and the client need no change.
It is plan item S2 of the one-click install plan. It serves VNC and RDP. RDP has no sound and no shared drive yet; see below.

Use it where there is no `guacd`: a hub that runs inside the application on a Mac. A hub in
Docker can keep `guacd` for both protocols, as today.

## Build

```sh
cargo build --release -p anima-gateway
```

The program is `target/release/anima-gateway`. It needs no file, no database and no
environment variable.

## Run

```sh
anima-gateway listen 127.0.0.1:4822
```

Then give the hub this address as its gateway:

```sh
ANIMA_GUACD=127.0.0.1:4822
```

The listener starts one worker process for each session. A fault in one session ends that
session only. At most 64 sessions run at one time; a connection beyond that gets the answer
"busy" and is closed.

## VNC here and RDP in `guacd`

RDP in the native gateway has no sound, no microphone and no shared drive. A hub that needs
them keeps `guacd` for RDP and uses the native gateway for VNC:

```sh
anima-gateway listen 127.0.0.1:4823 --other-gateway guacd:4822 --pass rdp
```

With `ANIMA_GUACD` set to the `anima-gateway` address, VNC sessions run in the native
gateway and RDP sessions go on to `guacd`. `--other-gateway` alone passes only the protocols
the native gateway does not serve. `--pass vnc` passes VNC.

## Devices

| Device | Settings in the device's VNC section |
| --- | --- |
| macOS Screen Sharing | The user name and password of an account on the Mac. The gateway then uses Apple's sign-in method and no other. |
| A VNC server with a VNC password (TigerVNC, x11vnc) | The password. Leave the user name empty: with a user name the gateway uses Apple's method only, and such a server does not have it. |
| A VNC server with no sign-in | No user name and no password. |

When the password (or, for a Mac, the user name) is not stored, the client asks for it when
the session opens.

### RDP devices

| Setting of the device | In the native gateway |
| --- | --- |
| Security `NLA` or `NLA-EXT` | The sign-in is made inside TLS before the session, with NTLM. The user name and the password must be there; the client asks for what is not stored. Kerberos is not used. |
| Security `TLS` | The account is sent after TLS; a device such as xrdp signs in with it or shows its own sign-in. |
| Security `ANY` | TLS or NLA, as the device picks. |
| Security `RDP` or `VMCONNECT` | Not served: the session ends with a gateway error. |
| The accepted certificate, "ignore certificate" | The gateway checks the device's certificate again before it sends the account (SEC-007). The hub has made the same check before. |
| Resize `display-update` | The device gets the size of the client's window. A device that does not change its screen is shown as a smaller picture. |
| Resize `reconnect` | The picture is scaled to the window; the gateway does not reconnect. |
| Audio out, microphone, shared drive, upload, download | Not there. The switches have no effect. |

The key table is the US keyboard. Text is right for every layout of the client, because a
character that the table cannot make goes as a Unicode key; shortcuts with Control, Alt or
the logo key use the US positions. Set the device's keyboard to US.

A screen larger than the client's window is sent as a smaller picture of it. A Mac with
several displays is one screen.

## Limits and security

- The gateway has no authentication, as `guacd` has none (GW-003). Bind it to a loopback
  address, or to a network that only the hub can reach (GW-002). Any program that can
  connect to its port can open sessions and can take all 64 session places.
- A VNC session with a VNC password or with Apple's method is not encrypted after the
  sign-in. The client shows the "VNC without TLS" badge (RA-020).
- The gateway takes the device as an IP address only. The hub resolves and checks the
  device's host (R-032) before it sends the address.
- No clipboard goes from the device to the client when copy is off for the device, and none
  goes to the device when paste is off (RA-033).
- The RDP library (IronRDP) does not clear the account from memory when it has signed in.
  One worker process holds one session and ends with it.
- Not built: the pointer image as a Guacamole cursor (the device draws the pointer),
  audio, file transfer, the Tight encoding, and the newer Apple sign-in types 33, 35 and 36.

## Log

The program writes to standard error: the address it listens on, and one line for each
session that ends, with the reason in fixed words. No line holds a device's text, an
address of a device or an account value (LOG-007).
