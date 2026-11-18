# Link setup guide (`anima-link`)

This guide runs the link by hand: an encrypted connection from a device to a hub, with no VPN
client, no account and no administrator rights. Plan: `docs/plans/one-click/one-click-plan.md`, Section 4.1
(LNK-001 to LNK-008, and the proposed LNK-009). Source: `link/`.

**Status.** `anima-link` is a program that runs by itself. The hub has a link mode for it
(plan item S5, Section 7): the hub then names each device in its rules. Nothing starts the two
programs together yet (plan items S4 and S6); this guide starts them by hand. A service other
than the hub learns nothing about the device from the link, and every connection reaches it from
`127.0.0.1`: put only a service with its own sign-in there.

Rules for this guide:

- Each code block starts with a comment that says where it runs.
- Words in `<ANGLE_BRACKETS>` are values you fill in.
- An invite and a hub address are secrets. Never paste one into chat, a note, a ticket or a
  commit. `anima-link` reads an invite from standard input and never takes one as an argument,
  because other users of a computer can read the arguments of a process.

## 1. Build

```sh
# On each computer, or once on a Mac: the script builds for macOS and Linux.
link/build.sh          # writes link/bin/anima-link-<os>-<arch>, 15 to 17 MB each
```

Go 1.27 is necessary to build. Nothing is necessary to run.

## 2. Start the hub link

```sh
# On the hub computer.
anima-link hub -state <HUB_STATE_DIR> -target 127.0.0.1:<PORT>
```

- `-target` is the path of the hub's socket (Section 7), or the loopback address of another
  service. The link refuses any other address.
- `<HUB_STATE_DIR>` holds the hub's key, its list of devices and its open invites. Only your user
  can read it; `anima-link` refuses a directory that others can reach. Keep its path short (a
  socket is made in it).
- The first start chooses a relay and saves it. The hub's address contains its key and its relay,
  so it stays the same at each later start (LNK-001).
- The hub link needs the network at its start: it sends a message to itself through the relay.
  If the relay does not carry it, the start fails with "the relay does not carry this hub's
  messages". The relay is then not reachable, or it refuses this hub's key.

Relays. With no `-relay`, the hub uses the nearest of Tailscale's shared relays. A relay cannot
read traffic, but Tailscale can record which keys and addresses connect, and the shared relays
have a rate limit. A relayed path is slow for remote display (about 4 to 9 Mbit/s when the relay
adds 40 ms; spike report `docs/plans/one-click/spike-link.md`, on its branch). To use a relay
of your own, start the first time with `-relay <HOST>[:<PORT>]`. The Headscale relay of the VPN
setup does not work as it is: it accepts only the keys of its own network.

## 3. Add a device

```sh
# On the hub computer. The answer holds the invite: a secret, valid for 15 minutes, for one use.
anima-link ctl -state <HUB_STATE_DIR> invites.make '{"name":"<DEVICE_NAME>"}'
```

Give the invite to the person at the device by a private channel.

```sh
# On the device. Type or paste the invite, then press Return.
anima-link join -state <DEVICE_STATE_DIR>
```

`join` prints the fingerprint of the device's key. Compare it with the hub's list:

```sh
# On the hub computer.
anima-link ctl -state <HUB_STATE_DIR> devices.list
```

If the fingerprint in the list is a different one, another party used the invite: revoke that
device (5) and make a new invite. A wrong, used or expired invite gets no answer; `join` then says
that the hub did not answer. A hub that is off gives the same message.

## 4. Use the link

```sh
# On the device.
anima-link device -state <DEVICE_STATE_DIR> -listen 127.0.0.1:<LOCAL_PORT>
```

`127.0.0.1:<LOCAL_PORT>` on the device now reaches the service on the hub computer.

```sh
# On the device: the path is "direct" or "relayed" (LNK-006).
anima-link ctl -state <DEVICE_STATE_DIR> status
```

`devices.list` on the hub shows the path of each device, and the relay's name when it is relayed.
It also shows `seen`: the time of the last sign of the device (HMD-047). The device link keeps one stream
open to port 446 of the hub link (the reverse port) and writes the line `here` on it each 15 seconds. A device
that gave no sign since the hub link started has no `seen`. A device link that is older than the reverse port
gives no sign, and a hub link that is older refuses the stream: the device link then tries again each minute
and its local port is not affected.

What the device link does by itself:

- It pings the hub each 15 seconds. This keeps the link ready, and it connects again after a hub
  restart (about 15 to 30 seconds).
- It renews its key in the last 30 days of the key's 180 days (LNK-005). The sessions that are
  open at that moment end once. A device that was off for the whole 30 days needs a new invite.

## 5. Remove a device

```sh
# On the hub computer. <KEY> is the "key" of the device in devices.list.
anima-link ctl -state <HUB_STATE_DIR> devices.revoke '{"key":"<KEY>"}'
```

The device's open sessions end in less than a second, and its key cannot connect again (LNK-004).

## 6. Change the relay

A different relay is a different hub address. Two steps let the connected devices follow:

```sh
# On the hub computer. Step 1: tell the connected devices the address that the hub will have.
anima-link ctl -state <HUB_STATE_DIR> relay.announce '{"relay":"<HOST>[:<PORT>]"}'
# Wait until each device has been connected once (a device reads the notice each 5 minutes).
# Step 2: change. The hub link stops and does not start by itself: start it again as in 2.
anima-link ctl -state <HUB_STATE_DIR> relay.switch
```

After step 2 the hub has a new address. If you use a domain name, put the new address
(`address.show`) in the TXT record.

If the hub link cannot start on the new relay (for example the name was wrong), no verb can run.
Start it once with `-relay <HOST>[:<PORT>] -replace-relay` to take a relay that works. The
devices that learned the announced address cannot follow this change.

A device that gets no answer at the old address tries the announced address after about 45
seconds. It moves only when the hub itself answers there through the tunnel. A device that was off during step 1 does not know the new address. It needs a new
invite, or a domain name:

- Optional, off by default: put the hub address (`anima-link ctl ... address.show`) in a TXT
  record named `_anima-link.<NAME>`, and join each device with `-hub-name <NAME>`. A device then
  finds a hub that moved.
- **Warning.** DNS is public. An address in DNS shows the hub's key, its pre-shared key and its
  relay to everybody. The list of permitted devices is then the only protection. The hub still
  refuses each key that is not in its list.
- A device moves only to an address that has the key and the pre-shared key of the hub that it
  joined. So a false record in DNS cannot send a device to another hub; it can only fail to help.

## 7. The hub behind the link

The hub has a link mode (plan item S5). In it the hub's rules name a device, not a network
address.

```sh
# On the hub computer. The folder must be the hub user's, with mode 0700.
ANIMA_LINK_SOCKET=<FOLDER>/link.sock ANIMA_LISTEN=127.0.0.1:<PORT> anima-hub
anima-link hub -state <HUB_STATE_DIR> -target <FOLDER>/link.sock
```

- The hub and `anima-link` must run as the same user. Either can start first.
- Do not set `ANIMA_PERMITTED_NETWORKS`. The hub permits this computer (loopback) and the
  devices of the link. The hub refuses to start on an address that is not loopback.
- Each device has a link address, for example `fd41:4e49:4d41:5acc:…`. `devices.list` shows it
  as `address`. The hub's audit log, its event log and the list of builders show the same
  address. It is a name, not a network address: no packet goes to it.
- A device keeps its address when it renews its key. A builder that the device enrolled stays
  bound to it (BLD-024).
- A key that left the list (revoked, or replaced by a renewal) is never permitted again. A
  revoked device that joins again must first make a new key: use a new state folder on the
  device. It then gets a new address, and what the hub bound to the old address stays unused.
- **Always give the socket as the target of a hub.** With a loopback target the hub cannot tell
  the devices apart: each device looks like this computer, a builder token works from each
  device, and all devices share one request limit. A loopback target is only for a service
  that needs no device names.
- Each local user of the hub computer can connect to the hub's loopback port. They must still
  sign in.
- `anima-link` connects only to a socket that its own user owns, in a folder that no other
  user can reach or replace. It writes the cause to its log when it refuses.

The metrics daemon of a device uses the link too (K8 Section 1):

```sh
# On the device. The port is the -listen port of `anima-link device`, 1024 or higher.
anima-daemon enrol - --hub http://127.0.0.1:<LOCAL_PORT>
```

- Give `anima-link device` a fixed `-listen` port on a device that runs the daemon. Without
  `-listen` the link uses a different port at each start. The daemon keeps its token for that
  origin, port included, and refuses to send it to another.
- Keep `anima-link device` in operation while the agent runs. While the link is stopped, a
  program of any local user of that device can listen on the port, and it then receives the
  daemon token. Nothing shows that this occurred. If you think that it did, revoke the token
  (`revokeDaemonToken`) or enrol the daemon again: the new token replaces the old one.

## 8. Limits and known faults

What the operator accepts with this version, the most important first:

1. With a domain name, the list of permitted devices is the only protection of the hub.
2. Each local user of a device can use that device's local port, and so reach the service.
3. A relay can always stop the link. It cannot read it.
4. A stolen device renews its key like any other, until you revoke it.

Other limits:


- The clock of the hub computer decides when keys and invites expire. A clock that is set forward
  expires every device.
- The first connection of a device is sometimes 5 to 10 seconds late (found in the spike; the
  cause is in the library and is not known).
- No more than 128 open connections for each device and 1024 for all devices.
- `anima-link` uses an unreleased version of the Tailcat library. See `link/UPSTREAM.md`.
