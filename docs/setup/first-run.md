# The first run of the desktop app

Version 1.0. Date: 2026-11-15. Status: **Draft. The steps follow the code of plan item S6. A
release of the application does not carry the hub programs yet, and the run-through on a clean
computer (task T11) has not happened.** This guide is a procedure. It adds no requirement.

Write: Simplified Technical English (design spec Section 0). Plan:
[s6-first-run-tasks.md](../research/one-click/s6-first-run-tasks.md). Requirements: FRN-001 to
FRN-009 ([s6-spec-delta.md](../research/one-click/s6-spec-delta.md)).

Related guides: [local-hub.md](local-hub.md) (the program that runs the hub),
[link.md](link.md) (the link).

## 1. The first start

When no hub is chosen, the app shows "Welcome to Anima.ing" and three choices (FRN-001).

| Choice | What it does |
| --- | --- |
| Sign in | Uses the hub that Anima.ing hosts for you. **This is not yet available.** The choice is grey and says "Not yet available." |
| Join a hub | Connects this computer to a hub with an invite from it. See Section 3. |
| Create a hub on this computer | This computer runs the hub, its database and its link. Other devices join it. See Section 2. |

Under "Sign in" there is a link, "Use a hub address". Use it only when you have a hub of your own on
a server. This guide does not cover that hub.

- "Join a hub" and "Create a hub on this computer" work in the desktop app only. A browser shows
  "Only the desktop app can join a hub." and "Only the desktop app can run a hub."
- "Create a hub on this computer" is grey with "This build does not carry the hub programs." when
  the build has no hub programs. A debug build (`bun tauri dev`) has none. See
  [local-hub.md](local-hub.md), Section 6, for `ANIMA_DEV_PROGRAMS` and `ANIMA_DEV_TREE`.

## 2. Create a hub on this computer

Choose "Create a hub on this computer". The page shows six steps ("Step 1 of 6" and so on). Each
step needs the one before it.

### 2.1 Step 1: Install the hub

1. Wait. The screen says "Anima.ing installs the hub, its database, its gateway and its link on this
   computer, then starts them."
2. Read the state line. The first start can take a few minutes.
3. When the state is "Ready.", the next step shows by itself.

If the install fails or the hub is blocked, the screen shows the reason and a "Try again" button.
See Section 8.

### 2.2 Step 2: Make your account

1. Enter a user name. Use 1 to 64 letters, digits, dots, underscores or hyphens. Start with a letter
   or a digit.
2. Enter a password of at least 12 characters.
3. Enter the password again.
4. Select "Continue".

### 2.3 Step 3: Add Anima.ing to your authenticator app

TOTP is required. There is no "skip" (FRN-004).

1. Scan the QR code with your authenticator app. Or type the key that the screen shows into the app.
2. Enter the 6-digit code that the app shows.
3. Wait. The screen says "Saving the account".

The hub saves the account and the key only when the code is correct. It then records that code as
used. The next step needs a new code.

If the code does not match, the screen says so. Wait for the next code, then enter it. Select "Back"
to change the name or the password.

### 2.4 Step 4: Sign in to your hub

1. Wait for the screen to check the hub address on this computer. The app sends nothing to the
   address until it checks that a program of your user listens there (F-10).
2. Wait for the next code in your authenticator app. The code that made the account cannot sign in.
   The screen shows how many seconds are left.
3. Enter the next code.

The app signs in with the account that you made and keeps the access token in the key store.

If the computer already had an account, this step asks for the user name and the password of that
account first. It then asks for a code. It does not make a second account.

### 2.5 Step 5: Save a copy of the hub key

The hub key is the file `vault.key`. It seals the device credentials and the authenticator keys.
Without it, the hub cannot open its data.

1. Select "Save a copy of the hub key".
2. Choose a place in the save dialog. Choose a place away from this computer, with your backups.

The app writes the copy (mode 0600). The web page never sees the key. If you select "Skip", you can
save the copy later in Settings, "This computer's hub", "Hub key".

A copy of the key helps only with a copy of the database. See Section 5.2.

### 2.6 Step 6: Invite your next device

1. Enter the name of the next device. The hub lists the device by this name.
2. Select "Make an invite".
3. Show the QR code to the other device, or copy the text. The screen shows the time that is left.
4. Select "Make another" to make an invite for one more device.
5. Select "Done".

An invite works once. It expires after 15 minutes (LNK-003). The invite is a secret. Do not put it
in a chat or a ticket.

When Stay awake is off, this step also says: "This computer is the hub. Its checks stop while the
computer sleeps." See Section 6.

### 2.7 What happens on the computer

- The app starts the program `anima-local-hub`. It runs as a background program of your user:
  - macOS: a per-user LaunchAgent with the label `ing.anima.hub`
    (`~/Library/LaunchAgents/ing.anima.hub.plist`).
  - Linux: a systemd user unit, `anima-hub.service` (`~/.config/systemd/user/anima-hub.service`).
- The hub starts at login. It runs when you close the app window. On Linux it runs while you are
  signed in.
- The program starts the database, the gateway, the hub and the link, in that order. It stops them
  in the reverse order.
- The data folder is the hub root. Only your user can open it (mode 0700).
  - macOS: `~/Library/Application Support/ing.anima.desktop/hub`
  - Linux: `$XDG_DATA_HOME/anima/hub` (default `~/.local/share/anima/hub`)
- The hub root holds the database, `vault.key`, the link's state (`link/`) and the logs (`log/`).
  [local-hub.md](local-hub.md), Section 1, lists each path.
- The operator account is made on this computer only, by the hub's own commands. The secrets pass
  on standard input. No hub API makes an account (FRN-003).
- The app saves the hub address `http://127.0.0.1:8443`. The app permits `http:` for the loopback
  address only (F-1).

**The hub opens no inbound port on a public interface.** The hub, the gateway and the database
listen on loopback or on Unix sockets. The link holds one UDP socket for WireGuard. It answers only
the keys of permitted devices (LNK-002). It serves no TCP port.

The gateway has no sign-in. Any program of this computer can ask it to connect to a VNC host. Do
not use a hub on a computer that other people use. See [local-hub.md](local-hub.md), Section 5.

## 3. Join a hub with an invite

Use this on a second computer. The hub must run, and its link must be up.

1. On the hub computer, open Settings, "Devices on the link". Select "Invite a device". Or use
   `anima link invite <name>` (Section 4).
2. Copy the invite text. It starts with `anima-invite.1.`.
3. On the new computer, choose "Join a hub".
4. Paste the invite into the field.
5. Select "Join". The screen says "Joining. This can take a minute."
6. Read the screen "This device joined the hub". It shows the key of this device.
7. Compare that key with the device list on the hub. If the hub does not list the key, the invite
   did not come from your hub. Select "Leave this hub" in Settings, then ask for a new invite.
8. Select "Continue to sign in". Sign in with the user name, the password and a code of the
   operator account.

Rules for an invite:

- An invite works once.
- An invite expires after 15 minutes.
- The hub gives no answer to a second use. The app then says: "The hub did not answer: the invite is
  used, expired or wrong, or the hub is off." Ask for a new invite.

Each join makes a new device key. A device that the hub revoked joins again with a new invite. No
other step is needed (FRN-005).

### 3.1 The join succeeds but the finish fails

After the join, the app removes the saved token of the link address, then saves the hub address.
If one of these steps fails, the screen shows "This device joined the hub" and a message:

- "This device joined, but the key store kept an old access token for the link address. Retry."
- "This device joined, but the link gave no address this app can use. Retry."

The invite is now used. Do not paste it again.

1. Select "Retry". The button repeats only the steps after the join.
2. When it works, select "Continue to sign in".

If the join itself is refused before the hub sees the invite, the field keeps your text. Correct
the cause, then select "Join" again. Refusals after the hub may have used the invite clear the
field.

## 4. Devices on the link

The link permits only the devices in its list (LNK-002). The operator manages the list on the hub
computer, or from any signed-in client of the hub (FRN-009).

### 4.1 In Settings

Open Settings, "Devices on the link". The section shows only on a hub with a link.

- The list shows each device with its name, its key and how it reaches the hub: "Direct",
  "Relayed through <relay>" or "Not yet connected" (LNK-006).
- Select "Invite a device" to make an invite (Section 2.6).
- To revoke a device, select "Revoke" on its row. Confirm. The device leaves the link at once and
  its open sessions end. It can join again only with a new invite.

### 4.2 With `anima link`

Sign in first with `anima auth login`. Use a login token. An API key gets `FORBIDDEN` for `invite`
and `revoke`.

| Command | What it does |
| --- | --- |
| `anima link devices` | Lists the devices: name, fingerprint, address, path, expiry, streams and key. Add `--json` for the full list. |
| `anima link invite <name>` | Makes an invite. It shows the invite once, on standard output. Use `--to-file <path>` to write it to a file (mode 0600) instead. |
| `anima link revoke <key>` | Revokes the device with that key, after a confirmation (or with `--yes`). It prints how many streams closed. |

Use the key from `anima link devices`. If the hub has no link, each command gets `UNAVAILABLE`.
The MCP tool `anima_link_devices` lists the devices. There is no MCP tool for the invite or for
revocation (MCP-004, MCP-005).

### 4.3 On a joined device

Settings, "Hub", shows "Link to the hub": "Up: direct", "Up: relayed", "Starting.", "Stopped." or
"Down". "Leave this hub" stops the link of this device and removes its device key. The device then
needs a new invite.

## 5. Recovery

**Recovery is local only (F-8).** You do it on the hub computer, in the app. A joined device, an
API key or a login token cannot reset the password or the TOTP secret.

### 5.1 A lost TOTP device, or a lost password

1. On the hub computer, open Settings, "This computer's hub".
2. In the row "Account", select "Recover the account". The hub must be "Ready".
3. Enter a new user name and a new password.
4. Scan the new QR code with your authenticator app. Enter the code.
5. A native dialog shows on the computer: "Recover the account". It says that every sign-in ends, on
   every device. Select the button that continues only if you asked for this yourself.
6. The hub replaces the account in one step. Every token of the operator stops. The app signs in
   again.

Script in the web page cannot answer the native dialog (F-9). Each replacement needs its own
confirmation.

**If you cancel the dialog, wait 60 seconds.** In that time the app shows no dialog. It answers
"Wait a minute before you try again." Then try again.

The audit log records the change (LOG-005).

Away from the hub computer, you cannot sign in without the TOTP device. There are no recovery
codes (OR-S6-2, option a). Wait until you are at the hub computer.

### 5.2 A lost hub key (`vault.key`)

If `vault.key` is missing and the database is not new, the hub is "Blocked". The program makes a new
key only for a new database. A new key cannot open what the old key sealed. The screen says: "Blocked:
the hub key vault.key is missing. Put your copy of vault.key back in <hub root>, then start the hub."

The ruled options are these (OR-S6-1):

1. **Put the key back.** This is the option that the app offers now.
   1. Copy your saved `vault.key` into the hub root. Set its mode to 0600.
   2. Select "Start" in Settings, "This computer's hub".
2. **No copy of the key.** Start a new hub: move the old hub root away, then run the first run again. The new hub has none of the data of the old hub. Do this only when you accept that loss.
3. **Reset the sealed data.** This option is not built. It is planned as an item of the backup plan
   (S9). It would make a new key, remove every sealed value and keep the devices, the checks and the
   history.
4. **A copy of the key in the macOS Keychain or the Secret Service.** This is not built. S6 builds
   the "Save a copy of the hub key" step instead.

Protect your copies:

- A Time Machine backup holds the database, `vault.key` and the link key together. FileVault and an
  encrypted backup disk protect them.
- A stolen hub computer is protected only by FileVault.
- A backup of the database is of no use without `vault.key`.
- Back up the `link/` folder with the database. A new link key is a new hub address. Each device
  must then join again.

### 5.3 A lost link key

If `<hub root>/link` is lost, the link makes a new key and a new hub address at its next start. Each
device joins again with a new invite (F-7).

## 6. Stay awake

When this computer is the hub and Stay awake is off, the checks of the hub stop while the computer
sleeps (AWK-005). The app says so in two places: the last step of the first run, and Settings, "Stay
awake".

The app treats the computer as the hub only when the hub is installed here and the app uses its
address (FRN-008). A joined device is not the hub, although it also uses a loopback address.

To change it:

1. Open Settings, "Stay awake".
2. Choose a level.
3. On a Mac, follow the one-time set-up that the section shows. macOS asks for an administrator
   once.

## 7. Start, stop and state

Settings, "This computer's hub", shows on the hub computer only. It has three rows.

| Row | What it shows or does |
| --- | --- |
| State | The state of the hub and of its link. "Stop" stops the hub. "Start" starts it. |
| Hub key | "Save a copy of the hub key". See Section 2.5. |
| Account | "Recover the account". See Section 5.1. |

The states of the hub:

| Text | Meaning |
| --- | --- |
| Not installed. | There is no hub on this computer. |
| Starting. | The first start can take a few minutes. |
| Ready. | The hub serves. "The database recovered after an unclean stop" means that nothing saved is lost. |
| Blocked: ... | The hub cannot start until you act. See Section 8.2. |
| Stopping. | The hub is stopping. "Start" waits. |
| Stopped. | "Start" starts it again. It also starts at the next sign-in to this computer. |
| Failed: <program> ended. | The unit starts the hub again after 10 seconds. |

The states of the link:

| Text | Meaning |
| --- | --- |
| The link is starting. | Devices cannot join yet. |
| The link is up: devices can join. | The link works. |
| The link is down; it tries again. This computer still reaches the hub. | The link needs its relay, so with no network it ends. |

**The link restarts on its own** (F-6). The program starts it again after 10 seconds, then 20, 40
and then each 60 seconds. The hub stays "Ready". You do not select anything.

"Stop" stops the hub until you select "Start" or sign in to the computer again.

## 8. Troubleshooting

### 8.1 Codes of the hub on this computer

The screen shows the text. These are the codes that the app's shell gives.

| Code | The screen says | Meaning and action |
| --- | --- | --- |
| `caller` | Only the main window can do this. | A window other than the main window asked. Use the main window. |
| `unavailable` | This computer cannot run the hub from the app. | The system cannot run the hub. |
| `not-carried` | This build does not carry the hub programs. | The build has no hub programs. Install a release that carries them. |
| `busy` | Another change to the hub is running. Wait for it to finish. | Wait. During a recovery, `busy` means the 60-second wait after a cancel (Section 5.1). |
| `no-home` | The app could not find your home folder. | The app cannot find your home folder. Check your user account. |
| `timed-out` | The hub did not answer in time. Try again. | Select "Try again". |
| `failed` | The hub programs did not finish. The application log has the cause. | Read `log/local-hub.log` in the hub root, and the application log. |
| `argument` | The hub refused the form of this request. | At the account step: use 1 to 64 letters, digits, dots, underscores or hyphens, starting with a letter or a digit. |
| `refused` | The hub refused the account. Check the password and the code. | The code did not match. Wait for the next code. |
| `cancelled` | Recovery was cancelled. Nothing changed. | You cancelled the native dialog. Wait 60 seconds. |
| `no-key` | The hub has no key file yet. | There is no `vault.key`. See Section 5.2. |
| `destination` | The key could not be written to that place. Choose another folder. | Choose another place in the save dialog. |

### 8.2 A blocked hub

| Text | Action |
| --- | --- |
| Blocked: the hub key vault.key is missing ... | See Section 5.2. |
| Blocked: the database is of another PostgreSQL version than this Anima.ing carries. | See [bundled-database.md](bundled-database.md), Section 6. |
| Blocked: another program runs the hub database. Stop it, then start the hub. | Stop the other `anima-database`, then select "Start". |

### 8.3 Codes of the device link

| Code | The screen says | Meaning and action |
| --- | --- | --- |
| `caller` | Only the main window can do this. | Use the main window. |
| `not-carried` | This build does not carry the link program. | Install a release that carries it. |
| `busy` | Another change to the link is running. Wait for it to finish. | Wait. A join or a leave runs. |
| `no-home` | The app could not find your home folder. | Check your user account. |
| `bad-invite` | This is not an invite. Paste the whole invite text, which starts with anima-invite.1. | Paste the whole text. An invite is at most 4 KiB. The field keeps it. |
| `no-answer` | The hub did not answer: the invite is used, expired or wrong, or the hub is off. | Check that the hub runs. Ask for a new invite. |
| `refused` | The hub refused this device. Ask for a new invite. | Ask for a new invite. |
| `folder` | The app could not write the link folder. | Check the rights of your user on the app's data folder. |
| `failed` | The link did not finish. The application log has the cause. | Read the application log. |
| `unavailable` | Only the desktop app can join a hub. | Use the desktop app. |

### 8.4 What the sign-in step says

| Text | Action |
| --- | --- |
| The hub refused the code. Wait for the next code: the code that made the account cannot sign in. | Wait up to 30 seconds. Enter the next code. |
| The hub did not answer. Wait a moment, then try again. | Check that the hub is "Ready". |
| The hub could not sign you in. Try again. | Try again. |
| Signed in, but the access token could not be kept in the key store. | Unlock the key store of your system. Sign in again. |
| The hub on this computer is not ready. Wait for it, then try again. | The hub was not "Ready" after 15 seconds. Wait, then select "Try again". |

### 8.5 When the app will not send to the hub address

The app sends no password and no token to a loopback address until it checks who listens there
(F-10). The screen then shows one of these.

| Title | Action |
| --- | --- |
| Another program holds the hub's port | Stop that program. Check again. |
| Nothing answers at the hub address | Start the hub on this computer, or the link to your hub. Check again. |
| The hub address could not be checked | Check again. |
| The link to the hub is starting | Check again in a moment. |
| The link to the hub is not up | On a joined device: "Another program holds the link's port <port>" means close that program. "The link ended" means check the network of this computer. The link starts again in 10 seconds. |

### 8.6 The hub says that the link socket path is too long

A socket path has at most 103 bytes. If `<hub root>/link/control.sock` is longer, the link does not
start and its state is "down". `anima-link.log` gives the path and its length. See
[local-hub.md](local-hub.md), Section 1.

### 8.7 The commands behind the screens

For the hub's own commands, see [local-hub.md](local-hub.md), Section 7: `anima-local-hub account
status`, `totp-new <name>` and `create <name>`. Each one needs the hub to be "ready". Exit code 20
means that the hub is not ready.

`anima-hub operator set` and `anima-hub operator totp <name> --disable` do not open a hub on the
link: in the link mode, the hub refuses sign-in for an operator with no TOTP secret.
