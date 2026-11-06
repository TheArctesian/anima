# Proposal: first-run setup in the app (no SSH to create the account)

From: the orchestrator. The operator chose this in chat on 2026-11-05 ("yes first run setup")
instead of an open sign-up page.
Status: a proposal for a ruling and for issue #1 (R-007). Not Phase 1 scope (contracts frozen, CON-001).

## Problem

The single operator account (OD-2) can be made only on the VPS: `anima-hub operator set <name>`
through `docker compose exec` over SSH, then `anima-hub operator totp <name>`, which prints an
`otpauth://` link to type into an authenticator (WP-05 D-9). A new operator needs SSH, sudo and
a terminal before the app does anything.

## Why not a sign-up page

The hub holds every device credential (OD-9). An open "create account" form lets the first
person who reaches it take the hub. VPN membership does not protect it: every VPN node,
including a site router, can reach `hub.anima.ing`. The model also has one operator and no roles,
so a second account could never be limited. One account and no sign-up stay.

## Proposal

1. **Setup state.** While the `operator` table is empty, the hub is "unclaimed". `GET /health`
   stays as it is. One new unauthenticated GraphQL field, `setupRequired: Boolean!`, tells the
   client which screen to show. When an operator exists it answers `false` and nothing else
   changes.
2. **Setup code.** At start, an unclaimed hub makes a random setup code (for example 4 groups of
   5 base32 characters, about 100 bits). It keeps only an argon2id hash and an expiry of 1 hour
   in memory, and writes the code itself once to standard output as a separate line that is not
   JSON-logged, so it never goes through the log pipeline into files or shipping (LOG-007,
   SEC-009). `anima-hub setup-code` on the VPS makes a new code and cancels the old one, for when
   the first one expired. A hub restart also makes a new one. Wrong codes count toward the
   sign-in throttle.
3. **Create account screen.** The client shows "Set up Anima" instead of "Sign in" when
   `setupRequired` is true. Fields: setup code, username, password (with the same rules as
   `operator set`). The hub checks the code and then, in one transaction, does what
   `operator set` does today and marks the hub claimed. The code is gone after one use.
4. **TOTP in the app.** The next step shows the TOTP secret as a QR code plus the key as text, and
   asks for one 6-digit code to confirm before saving it. That is the same rule as
   `operator totp`, with the secret shown in the app instead of a terminal. The setup session
   that gets this far is short-lived and can only enrol TOTP.
5. **After setup.** `setupRequired` answers `false` forever, the setup mutation refuses, and the
   client shows only "Sign in". An audit entry records the claim with the client IP (LOG-005).
   `anima-hub operator set` and `operator totp` stay as the recovery path on the VPS.
6. **Later, same pattern.** A "forgot password" flow could reuse it: `anima-hub setup-code
   --reset` on the VPS gives a code that lets the app replace the password and TOTP. This is not
   part of this proposal.

## Risks and answers

- **Someone on the VPN claims the hub first.** They would need the code, which only appears in
  the hub's standard output on the VPS. The window is the time between `docker compose up` and
  the operator's setup.
- **The code in `docker logs`.** Container stdout is kept by Docker's json-file driver (rotated,
  3 files). That is the reason for the 1-hour expiry and single use. A stricter option is to
  write the code only to a root-only file in the secrets folder and print its path instead. The
  operator decides which.
- **New unauthenticated surface.** One query and one mutation, both answering only while
  unclaimed. Inputs are bounded and throttled like sign-in. SEC-002 needs a stated exception
  for them.

## Needs

- A spec change: a setup-screen requirement in the access section, an exception to SEC-002 for
  `setupRequired` and the setup mutation, and the setup-code handling under LOG-007.
- A ruling on where the code appears (stdout once, or a root-only file).
- A contract change to K3 (access surface) for the new query and mutation, after Phase 1.
- An issue #1 checklist entry (R-007). This pairs with
  [in-app-enrolment.md](in-app-enrolment.md) (R-055): together they give "install the app,
  enter a code, done".
