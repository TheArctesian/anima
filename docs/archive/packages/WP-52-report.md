# WP-52 report — Deploy and infrastructure

Status: **done** (five tasks accepted). Package branch `claude/wp-52-deploy`; plan, decisions E-1 to E-6 and the
review log are in [WP-52.md](WP-52.md). No Rust or contract change, so every task ran gates 4–12 (R-068).
Nothing ran on the live VPS.

## Accepted tasks

| Task | SHA | Gates | Rounds | State |
| --- | --- | --- | --- | --- |
| T1 web-image (D-2, R-031) | 9b26fb10 | 4–12 | 2 | merged into phase-3 |
| T2 compose (K7, D-13, D-19, NET-001) | 66ce4a30 | 4–12 | 2 | merged into phase-3 |
| T3 egress (NOT-002) | 8d03fb64 | 4–12 | 2 | merged into phase-3 |
| T4 docs + T5 apprise-firewall (D-15, D-19, D-24, NOT-002, VPS-002, VPS-005) | 6e69de44 on 10f9045c | 4–12 | T4 2 + fixes reviewed; T5 2 | sent for merge |

Branch for T4 + T5: `wp/52-deploy/apprise-firewall` (rebased; the origin copy at cd36c290 is the old base).

## Live steps for the operator (VPS-005)

Nothing in the repository applies them. Each needs the operator's OK in chat and a row in
`operator-changes.md`. They are in `deploy/README.md`, "Phase 3 operator changes (WP-52)", in this order, each
with an undo:

1. Check the free disk space (D-14).
2. Keep the current image and compose file; take a backup (`anima-backup.service`, or `backup.sh`) and check
   the dump file.
3. Build the hub image on the Mac for amd64 and load it on the VPS.
4. Install the new compose file and the egress check.
5. Start the new hub (check `172.30.3.0/24` is free first). Undo: restore the step 2 dump, then the old image.
6. Run the egress check (443, 465, 587 from the hub).
7. Required before Apprise: install the Apprise firewall rules with `harden.sh --dry-run firewall`,
   `harden.sh firewall`, then `harden.sh confirm` from a new VPN SSH session within 180 s; check the table, the
   boot copy, `harden.sh check`, `br_netfilter` and `net.bridge.bridge-nf-call-iptables = 1`; run the probe
   (six URLs print 28, ntfy.sh prints 0).
8. Optional: start Apprise (`docker compose --profile apprise up -d`), only after step 7.

## Handoffs

- WP-45: refuse Apprise URLs with query keys outside a known list (`template=` makes Apprise fetch any URL).
  The firewall rule contains it today; this narrows it at the source.

## Should items for issue #1

- Load-flaky deploy test `localapi-server.test.ts` "endless upstream body gives 502 promptly"; passes alone.

## Could items for issue #1

- T5 C-4: nothing at run time notices lost Apprise rules. `harden.sh check` could flag a running `apprise`
  container while the live table lacks `ip saddr 172.30.3.10 drop`; if `nftables.service` fails at boot,
  Docker still starts Apprise.
- T1: the "only the build output" test matches the text `--from=web`, so `COPY --from=0`,
  `RUN --mount=…,from=web` and `install -d -m 0777` get past it; no test forbids `--platform` on the rust stage;
  the web stage's `--platform=$BUILDPLATFORM` is optional in the test regex; indentation and a second read of
  the Dockerfile in `compose-hub.test.ts`.
- T3: no test for `(unreachable)`; exit 142 exists only for the test shim; docker failing in the middle of the
  loop reads as "blocked".
