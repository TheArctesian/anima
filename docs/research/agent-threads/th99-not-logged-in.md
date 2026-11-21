# Record: "Not logged in" of Claude Code under the holder on a busy Mac (task TH99, 2026-11-21)

| Field | Value |
|---|---|
| Document | TH99 record: why Claude Code said "Not logged in" in a thread while the person was signed in |
| Date | 2026-11-21 |
| Versions measured | `claude` 2.1.311 (/opt/homebrew/bin/claude), macOS 27.3.0, arm64; the installed daemon of dev f65fd266b |
| Kind | Record. It adds no requirement (PLN-010). It is the cause for task TH99 of [agent-threads.md](../../proposals/agent-threads.md). |

No secret was read. The runs looked at which programs Claude Code started and for how long, read the
program file of Claude Code as text, and looked for the Keychain item without `-w`, which says whether the
item is there and prints no secret.

## 1. What was seen

1. Measured, 2026-11-21, about 09:3x UTC, load average about 76: a thread of Claude Code under the holder
   ended its first turn with "Not logged in · Please run /login". From a terminal with the same environment,
   `claude` was signed in.
2. Measured, 09:57 UTC, load average about 60: four threads of Claude Code were started at once. Each
   `claude` started `security find-generic-password -a <user> -w -s "Claude Code-credentials"`. Each of these
   took 14 to 22 seconds, then `claude` started it again. The threads said "starting" for about 3 minutes,
   then each came to "idle"; none said "Not logged in" this time.
3. Measured at the same time: the same item looked up from a terminal without `-w` answered in 0.02
   seconds. The `ps` of the holder also took 20 to 27 seconds for each call.

## 2. The cause

The program file of Claude Code 2.1.311 holds the code that reads its sign-in on a Mac (read as text):

- The main read runs `security find-generic-password -a <user> -w -s "Claude Code-credentials"` with a
  timeout of 2000 ms. A read that fails or ends at the timeout gives no sign-in.
- With no sign-in from the Keychain, it reads the plain file `~/.claude/.credentials.json`. On this Mac that
  file is from 2026-11-17 and is not the current sign-in.
- With neither, the turn ends at once with an `assistant` line whose `error` is `authentication_failed`
  and the text "Not logged in · Please run /login", then a `result` with the subtype "success",
  `is_error: true` and `terminal_reason: "api_error"` (measured with an empty configuration folder).
- When `CLAUDE_CONFIG_DIR` is set, the name of the item ends with 8 hexadecimal digits of a hash of the
  folder, so the runner cannot name the item.

Thus on a computer with a high load, the `security` program does not answer in 2 seconds, and Claude Code
says "Not logged in" though the person is signed in. Nothing of the holder or of its environment causes it:
`HOME`, `USER` and the login session of the Keychain are those of the person (the same `security` call
answers, only later). A start of several threads at once makes it more likely, because each `claude` reads
the Keychain at the same time.

## 3. What the runner does (TH99)

- The decoder of Claude Code ends such a turn as failed with the detail `authentication_failed` (it read it
  as completed before, because of the subtype "success").
- The runner then looks whether the item is in the Keychain, without `-w`, for at most 5 seconds, and adds
  a notice to the thread: when the item is there or the Keychain is slow, "you do not need to sign in; send
  the prompt again"; only when the item is not there, "sign in to Claude Code in a terminal (`claude`, then
  `/login`)"; when it cannot tell (a configuration folder, not a Mac), both cases.

## 4. Not done

- No automatic second try: a second prompt adds a second copy of the prompt to the session of Claude Code,
  so the person sends it again.
- The runner does not start the threads of Claude Code one after the other to make the reads fewer.
- Linux: Claude Code keeps its sign-in in a file there; not measured.
