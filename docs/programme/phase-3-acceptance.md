# Phase 3 acceptance matrix

Owner: WP-53. Criteria: [phase-3-plan.md](phase-3-plan.md) Section 5. Base: `phase-3` at e87571f8. Paths: `hub/` is `crates/hub/`, `client/` is `apps/client/src/`, `agent-core/` is `crates/agent-core/src/`.

Rulings that change the criteria: R-111 (sharing deferred), R-113 (session recording deferred, merged recording code removed by WP-48 T10). R-112 sets the preparation pace. This file is the preparation draft. Acceptance itself starts after WP-48 T10 and WP-49 merge and the operator supplies the test machines (plan Section 8, checkpoint 4).

Status words:
- **open**: nothing is recorded yet.
- **pass**: the evidence is in the repository, and every live step for the row is done.
- **fail**: a spec failure that is still open.
- **operator**: the repository part passes, and a live step remains that only the operator can do (steps are in "Operator steps").
- **withdrawn**: the criterion is not checked (the ruling is named).

Rulings R-111, R-112 and R-113 are committed in programme.md (R-113 at c90767f4). Main confirmed these readings:
- Criterion 14 is `withdrawn (R-111)`. Criteria 16, 17 and 18 are `withdrawn (R-113)`.
- Criterion 22 drops "share token" and "recording key". It keeps password, key, token, cookie, API key, notification secret, webhook URL and session content.
- Criterion 23 drops "on the watch page or in playback". It keeps "session tab open" and "nothing moves or blurs over a live display".
- No row cites recording or share code or tests. R-113 removes the recording code.

A test is cited as `file` `name`. Rust names are `fn` names. Client names are the exact title string of the `it(...)` call. A title with `%s` or `%j` is a parameterised title; cite it as written.

WP-53 scripts and tests (merged):
- T2 `quality/acceptance/phase-3.ts` (e2ab858b): criterion 24 (no `@pending` use in K1, `quality/baseline.json` is `{}`). Tests: `quality/tests/acceptance.test.ts`.
- T3 `deploy/acceptance/log-scan.ts` (d113760d): criterion 22, scan of the live hub log. Tests: `deploy/test/acceptance-log-scan.test.ts`.
- T4 `deploy/acceptance/probe.ts` (e34a2995): criteria 4, 12 and 13 against the live hub. Tests: `deploy/test/acceptance-probe.test.ts`, `deploy/test/acceptance-probe-verdicts.test.ts`, `deploy/test/acceptance-prometheus-text.test.ts`.
- T5 `hub/tests/it/acceptance/canary.rs` (ee1efa8d): criterion 22, whole-hub log canary.
- T6 `docs/research/motion/phase-3.md` (e45e00f7): criterion 23, motion audit.
- T7a (7ebf2aec): motion fixes M-1 (API key dialog) and W-1 (last session tab exit), criterion 23.
- T7b (1957c4cf): hub gap tests G-2, G-3 and G-10, criteria 7, 9 and 19.
- T7c (e87571f8): client gap tests G-1 and G-11 and the GPU shared memory chart, criteria 2 and 21.

## Matrix

| # | Criterion (short) | Spec IDs | How checked | Evidence to record | Status |
| --- | --- | --- | --- | --- | --- |
| 1 | The hub serves the web client at `https://hub.anima.ing/`. Password and TOTP sign-in in Safari and Chrome (macOS) and Firefox or Chromium (Linux). The browser holds only an `HttpOnly` cookie, no token in script-readable storage. | Section 1.3, CLI-001, SEC-001, SEC-002, D-2, D-3 | Hub serves the page: hub `tests/it/web_client.rs` `d_2_an_unknown_path_gets_index_html_with_a_web_root`, `d_2_post_graphql_still_reaches_graphql_with_a_web_root`; `src/web_client/tests/serving.rs` `d_2_the_root_serves_index_html`. Cookie only: hub `tests/it/browser/sign_in.rs` `d_3_sign_in_browser_sets_exactly_the_cookie_and_answers_without_the_token`, `d_4_sign_in_browser_needs_the_totp_code_once_enrolled`. Client: `client/routes/sign-in/page.test.ts` "D-3, E-4: a sign-in sends SignInBrowser with the form input, never Login, and loads /devices as a new page" and "CLI-003, LOG-007: after a good sign-in nothing secret is in the DOM, the console or web storage". Live in three browsers: steps P-3 and P-4. | Per browser: name and version, sign-in result, the cookie row from the developer tools (name, `HttpOnly`, `Secure`, `SameSite`), and "no token in local or session storage" | open |
| 2 | In the web client: device list, detail view, live status, logs panel, SSH and VNC sessions work. Desktop-only controls (updates, log folder, web view tab, pop-out, This Mac, System Settings) do not show. The updater is never loaded. | CLI-001, UPD-001, UPD-004, LOG-008, RA-001, D-5 | Hidden controls: client `lib/platform/platform.test.ts` "CLI-001: outside the desktop shell there are no desktop answers" and "CLI-001, UPD-001: outside the desktop shell every answer is no"; `lib/updates/tests/browser-client.test.ts` "UPD-001, CLI-001: with the browser answers a production build never checks and never loads the updater module"; `lib/updates/tests/settings-rows.test.ts` "UPD-006, BLD-020: the browser shows no Updates, log folder or Builds row"; `lib/web-interfaces/shell/client-shell.svelte.test.ts` "CLI-001: with the browser answers a tab never asks the desktop shell"; `lib/devices/add/this-machine.svelte.test.ts` "R-059, CLI-001: with the browser answers no Remote Login button is offered"; `lib/devices/screen/machine-prompt.test.ts` "R-060, CLI-001: with the browser answers this Mac is never read, asked about or offered, even inside a shell". No web-view tab or pop-out: `lib/sessions/strip/panes.test.ts` "WEB-003: a web tab has no pop-out menu and closes without ending a session". List and detail: `lib/devices/list.test.ts` "DEV-001 starts from a page and keeps the cursor"; `routes/devices/[id]/page.test.ts` "D-11: a status change of this device updates the pill; one of another device does not". Device page with the browser answers (G-1): `routes/devices/[id]/detail/browser-page.test.ts` "CLI-001, LOG-008: the logs panel renders this device's events", "CLI-001, D-11: the status pill renders, and a live event of this device updates it", "CLI-001, UPD-001, D-5: with %j no control is named for a desktop feature" (all five tabs; a text scan, it cannot fail through capability gating because the updates, log folder, This Mac and System Settings controls live on the Settings and add-device screens, covered by the row 2 tests above), "CLI-001, WEB-011: with $name the Web button opens $opens" (browser answers: a browser tab and no web view tab; desktop answers as positive control: a web view tab and no browser tab). Live data and sessions: steps P-5 and P-6. On the desktop app the update banner (UPD-004: "Anima <version> is installed", a "Restart to update" button, `role="status"`) is checked live in step P-17 (desktop only; main's ruling 2026-11-11). | Browser, device list screenshot, detail view with live status, the logs panel, one SSH and one VNC session (device, time, result), the list of desktop-only controls checked as absent | open |
| 3 | The Web button opens a new browser tab straight to the device, with `noopener` and `noreferrer`. The pinned fingerprint shows beside the button. | WEB-011, D-5 | Client `lib/web-interfaces/open/open.test.ts` "WEB-011: %s with %s opens one new tab with noopener,noreferrer and no Anima tab", "WEB-011: window.open is called before the first await, inside the click"; `lib/web-interfaces/open/pinned.svelte.test.ts` "WEB-011: shows the pin as the trust page writes it, under its label", "WEB-011: with two interfaces and one pin, the pin belongs to its own choice and the other has none". | Test run result (names above) | open |
| 4 | A cookie-carrying `POST /graphql` or WebSocket upgrade from another origin is refused. The cookie has `__Host-`, `Secure`, `HttpOnly`, `SameSite=Strict`. Sign out clears it and deletes the token. Static files carry the K7 headers. `CLIENT_ORIGINS` is unchanged and the desktop app still signs in. | SEC-002, API-003, D-3, D-4 | Another origin: hub `tests/it/browser/sign_in.rs` `a_4_sign_in_browser_from_another_origin_is_forbidden_and_not_recorded`; `tests/it/browser/cookie.rs` `d_3_the_cookie_signs_in_only_with_the_public_origin`; `tests/it/browser/socket.rs` `d_3_a_socket_from_another_origin_ignores_the_cookie`; `tests/it/graphql.rs` `another_origin_gets_no_cors_permission`. A cookie socket from another origin is refused with close 4403 at `connection_init` (the module comment of `tests/it/browser/socket.rs` states the rule; `d_3_a_socket_from_another_origin_ignores_the_cookie` proves it). Cookie attributes: hub `src/access/browser/tests.rs` `d_3_the_cookie_is_set_with_every_attribute_and_cleared_with_the_same`. Sign out: hub `tests/it/browser/cookie.rs` `a_8_logout_deletes_the_browser_token_and_clears_the_cookie`; `tests/it/browser/sign_out.rs` `a_8_logout_closes_a_cookie_socket_on_the_same_token`. K7 headers: hub `src/web_client/tests/headers.rs` `d_2_index_html_carries_the_headers_and_no_cache`, `d_2_an_immutable_asset_is_cached_for_a_year`, `sec_001_refusals_carry_the_headers_and_are_never_cached_as_immutable`. CORS and desktop sign-in: hub `tests/it/graphql.rs` `the_desktop_web_view_may_post_to_graphql` (sends one Tauri origin, no cookie); `tests/it/browser/sign_in.rs` `sec_002_the_desktop_app_still_signs_in_from_its_web_view`. Live hub: `deploy/acceptance/probe.ts` (T4, merged; K7 headers on `/` and one hashed asset, foreign-origin `POST /graphql` with a fake cookie, cross-origin WebSocket upgrade, CORS for the two Tauri origins only). Its tests are `deploy/test/acceptance-probe.test.ts` and `deploy/test/acceptance-probe-verdicts.test.ts`; the `CLIENT_ORIGINS` list is pinned by `acceptance-probe-verdicts.test.ts` "SEC-002 G-4: the client origins are CLIENT_ORIGINS of lib.rs" (G-4). | T4 output (pass lines per check, run date, hub origin) and the desktop app sign-in result against the live hub (P-4) | open |
| 5 | A Down and an Up notification go through webhook, ntfy, email and Apprise. None for a check in maintenance. No "Up" after Maintenance unless "Down" was sent. | NOT-001, NOT-002, Section 6.5 | Decision: hub `src/notifications/decision/tests.rs` `not_001_down_is_sent_once_and_up_once_after_it`, `not_001_degraded_after_down_is_back_up_and_never_sends_down`, `mw_6_5_up_after_maintenance_is_sent_only_when_down_was_sent_before`, `mw_6_5_nothing_is_sent_in_maintenance_and_no_up_follows_an_unsent_down`. Maintenance: hub `src/checks/scheduler/tests/status.rs` `not_001_a_down_beat_under_maintenance_queues_nothing`; `tests/it/notifications/checks/heartbeats.rs` `not_001_a_pushed_down_under_maintenance_queues_nothing`, `not_001_an_up_pushed_once_maintenance_starts_queues_nothing`. Each channel kind has one synthetic send, in hub `src/notifications/dispatch/tests/kinds.rs` `not_002_a_webhook_notification_is_posted_and_logged`, `not_002_a_ntfy_notification_is_posted_to_its_topic`, `not_002_an_email_notification_is_mailed`, `d_18_a_stateless_apprise_notification_is_posted_with_its_urls`; the live steps cover the real endpoints. Real endpoints: steps P-9 to P-11. | Per channel: the Down and the Up as received (time, channel), and "none received" during maintenance | open |
| 6 | Channels are selected per check. A channel "on by default" is on for each new check. | NOT-003 | Hub `tests/it/notifications/checks/check_field.rs` `not_003_a_new_check_notifies_the_channels_on_by_default`, `not_003_with_no_channel_on_by_default_a_new_check_notifies_none`; `tests/it/notifications/checks.rs` `not_003_the_channel_set_is_replaced_and_the_answer_lists_it`; `src/notifications/tests/observe.rs` `not_003_adopt_defaults_links_only_channels_on_by_default`. Client `lib/checks/form/channels/channels.svelte.test.ts` "NOT-003: a new check pre-selects the on-by-default channels and lists each with its kind". | Test run result | open |
| 7 | "Send test" on each channel returns a delivery result within 8 seconds. | NOT-004 | Hub `src/notifications/graphql/tests/test_send.rs` `not_004_a_test_notification_is_sent_at_once_and_logged`, `not_004_a_failed_test_is_answered_as_a_delivery_with_its_reason`, `not_004_a_test_to_a_silent_target_ends_within_the_time_limit` (the limit `TIME_LIMIT` in `hub/src/notifications/egress.rs` is 8 s; this test proves it for a webhook), and in the same file `not_004_a_test_to_a_silent_ntfy_server_ends_within_the_time_limit`, `not_004_a_test_to_a_silent_mail_server_ends_within_the_time_limit`, `not_004_a_test_to_a_silent_apprise_server_ends_within_the_time_limit`, `not_004_the_send_time_limit_is_8_seconds` (G-2). Client `lib/notifications/channels/list.svelte.test.ts` "NOT-004, MOT-007: while the hub tests, Send test keeps its label and focus, says it is busy and sends once". Real channels: step P-10. | Per real channel: the result shown and the elapsed seconds | open |
| 8 | With resend interval N, a check that stays Down sends again after each N Down heartbeats. | NOT-005 | Hub `src/notifications/decision/tests.rs` `not_005_still_down_is_sent_at_n_and_2n_down_beats_after_down`, `not_005_zero_never_resends`, `not_005_resend_of_one_sends_still_down_on_every_further_beat`; `src/notifications/tests/observe.rs` `not_005_observe_reads_the_checks_resend_setting`; `tests/it/notifications/checks.rs` `not_005_resend_beats_outside_zero_to_1000_are_refused`. | Test run result | open |
| 9 | One expiry notification per threshold (default 21, 14, 7 days) for each TLS certificate and each domain expiry check. | NOT-006, CHK-007 | Hub `src/notifications/decision/tests.rs` `not_006_one_notification_per_threshold_as_days_run_down`, `not_006_first_sight_at_five_days_sends_one_and_marks_every_crossed_threshold`, `not_006_certificate_and_domain_are_marked_and_sent_apart`, `not_006_a_renewed_certificate_drops_the_old_marks_and_starts_again`; `src/notifications/tests/observe.rs` `not_006_observe_queues_an_expiry_with_days_and_marks_it_once`; `tests/it/notifications/settings.rs` `not_006_the_defaults_read_back_and_save_again_unchanged`. Domain expiry check to a queued notice: `src/checks/scheduler/tests/domain.rs` `not_006_a_domain_check_beat_queues_one_notice_for_each_threshold_it_crosses`, `not_006_a_domain_first_seen_at_six_days_queues_one_notice` (G-3); the 7-day threshold itself: `src/notifications/decision/tests.rs` `not_006_a_threshold_is_crossed_only_when_fewer_whole_days_are_left`. | Test run result | open |
| 10 | A template changes the title and body. Only the fixed placeholders are accepted. Values are escaped per channel: JSON for webhooks, no line break in an email subject or a header. | NOT-007, D-21 | Hub `src/notifications/template/tests.rs` `not_007_every_placeholder_is_replaced_by_its_value`, `d_21_an_unknown_placeholder_is_refused_and_named`; `src/notifications/encode/tests.rs` `not_007_a_webhook_value_with_quotes_and_braces_stays_one_json_string`, `not_007_an_email_subject_built_with_hostile_values_stays_on_one_line`, `not_007_ntfy_title_never_holds_a_control_character`; `src/notifications/dispatch/tests/kinds.rs` `not_007_a_channel_template_words_its_notification`. Client `lib/notifications/template/template.test.ts` "D-21: a placeholder outside the list is refused and named". | Test run result | open |
| 11 | Every notification connection passes the target rule and the deny list for each resolved address, follows no redirect, verifies TLS. A deny-listed channel fails with a fixed message. Secrets are sealed, never returned, never logged. | P3-003, SEC-003, SEC-005, LOG-007, D-17 | Hub `src/notifications/egress/tests/http.rs` `p3_003_a_refused_address_gets_no_connection`, `p3_003_a_redirect_is_a_failure_and_is_not_followed`, `d_17_an_untrusted_certificate_fails_before_the_request`; `src/notifications/egress/tests/smtp.rs` `p3_003_a_refused_mail_server_gets_no_connection`; `tests/it/notifications/log.rs` `p3_003_a_test_to_a_refused_address_is_logged_unsent_and_audited`; `src/notifications/tests/sealing.rs` `sec_003_channel_secrets_round_trip_and_are_not_stored_in_clear`; `tests/it/notifications/channels.rs` `sec_005_a_webhook_reads_back_with_its_origin_and_header_names_only`; `src/notifications/dispatch/tests/secrets.rs` `log_007_no_secret_is_logged_recorded_or_delivered_by_a_failing_send`. Client `lib/notifications/channels/delivery.test.ts` "CR-14.5, NOT-004: every K1 reason has its own fixed sentence". | Test run result | open |
| 12 | `GET /metrics` gives Prometheus text with status, response time, uptime ratio and certificate days for each check. Labels hold no value that changes often. It refuses a request with no token. | INT-001, INT-002, D-24 | Hub `src/checks/exposition/tests/render.rs` `int_001_renders_every_family_with_help_type_and_values`, `int_001_status_values_follow_k7_and_paused_is_unknown`; `tests/it/checks/exposition/series.rs` `int_001_response_time_is_the_last_heartbeat_and_absent_without_one`, `chk_011_uptime_ratio_has_the_three_windows_from_the_aggregates`, `int_001_certificate_days_only_for_checks_with_a_certificate`; `tests/it/checks/exposition/labels.rs` `int_002_labels_are_the_k7_names_and_hold_no_status_time_or_address`, `int_002_labels_do_not_change_when_status_and_values_change`; `tests/it/bearer/metrics.rs` `d_24_int_001_metrics_without_a_token_answers_401_and_starts_no_build`, `int_001_a_read_key_a_write_key_and_a_login_token_scrape_prometheus_text`. Live hub text and labels: `deploy/acceptance/probe.ts` (T4, merged) with `--key-file`; the strict Prometheus reader it uses is tested in `deploy/test/acceptance-prometheus-text.test.ts` and its verdicts in `deploy/test/acceptance-probe-verdicts.test.ts`. Optional real scrape: step P-14. | T4 output; for the optional scrape, the Prometheus target state "up" and one query result | open |
| 13 | An API key has a scope and an expiry. The secret shows once and the hub keeps only its SHA-256. A read key cannot run a mutation. No key can manage keys, sign in a browser, start a session or open files. Revoke stops the key at once. Each step has an audit entry. | INT-003, LOG-005, D-23 | Hub `tests/it/api_keys/create.rs` `int_003_d_23_the_secret_is_anima_and_43_base64url_characters_shown_once`, `int_003_p3_004_only_the_sha_256_of_the_secret_is_stored`, `d_23_the_expiry_is_in_the_future_and_at_most_one_year_away`; `tests/it/api_keys/refused.rs` `d_23_a_read_key_cannot_run_any_mutation`, `d_23_every_refused_root_is_forbidden_for_an_api_key_in_every_spelling`, `d_23_refused_fields_with_valid_arguments_change_nothing`; `tests/it/api_keys/revoke.rs` `d_23_revoke_deletes_the_key_and_it_stops_at_once`, `log_005_create_and_revoke_each_leave_an_audit_entry_and_an_event`. Live hub: `deploy/acceptance/probe.ts` (T4, merged) with `--key-file` (a read key gets FORBIDDEN for one mutation and each refused root, with invalid arguments); the probe's list of the 30 refused roots is pinned to `API_KEY_REFUSED_ROOTS` by `deploy/test/acceptance-probe-verdicts.test.ts` "D-23 R-100: the refused roots are API_KEY_REFUSED_ROOTS of guard.rs". | Test run result; T4 output | open |
| 14 | Share link for a live SSH and VNC session. | RA-035, D-7 to D-10 | Not checked. Sharing is deferred (R-111). | none | withdrawn (R-111) |
| 15 | With a session limit of N, session N+1 is refused with CONFLICT. File tunnels do not count. | RA-036 | Hub `tests/it/sessions/start/limit.rs` `ra_036_a_device_at_its_limit_refuses_the_next_start_until_one_ends`, `ra_036_two_parallel_starts_at_a_limit_of_one_give_exactly_one_ticket`, `ra_036_files_tickets_hold_no_session_and_never_count`, `ra_036_a_limit_lowered_below_the_live_count_refuses_the_next_start`. The word "viewers" in the plan criterion has no meaning after R-111. | Test run result | open |
| 16 | Recording off by default, "Recording" badge, no plaintext. | RA-040, RA-008, D-11, D-13 | Not checked. Recording is deferred (R-113). | none | withdrawn (R-113) |
| 17 | Playback with `Guacamole.SessionRecording` and the asciicast player. | RA-041, D-12 | Not checked. Recording is deferred (R-113). | none | withdrawn (R-113) |
| 18 | Expired recording deleted with its key. | RA-040, D-14, D-15 | Not checked. Recording is deferred (R-113). | none | withdrawn (R-113) |
| 19 | Settings has: hub address and test (desktop), theme, data retention, notification channels, API keys, default check settings, in-app reduced motion. New checks use the defaults. A changed retention applies at the next daily run. | SET-001, Section 7, UI-005, TOK-011, HUB-005, MOT-043, D-25 to D-28 | Page order: client `routes/settings/page.test.ts` "SET-001, UI-005, MOT-043: the page is the title, then Account, then Appearance with Theme, Reduce motion and the two fonts, then Notifications, then Default check settings, then Data retention, then API keys". Hub address (desktop): `routes/settings/page.test.ts` "CLI-002, F-10: desktop shows Hub after Account and before Appearance, with the stored address and an empty status line", "CLI-002: Test connection shows that the hub answered, also when it refuses an unsigned caller". Theme and motion: `lib/theme/row.test.ts` "UI-005: each choice sets data-theme on <html> at once and is still chosen after a restart"; `lib/theme/motion-row.svelte.test.ts` "MOT-043: each choice sets data-motion on <html> at once and is still chosen after a restart". Defaults: hub `tests/it/checks/defaults.rs` `d_26_a_check_added_after_set_check_defaults_takes_the_new_settings`; client `lib/checks/defaults/prefill.test.ts` "D-26: a new check starts from the stored defaults". Retention: hub `tests/it/retention/periods.rs` `hub_005_periods_return_the_stored_values`; `src/upkeep/tests.rs` `hub_005_each_domain_drops_by_its_own_period`, `hub_005_a_period_set_through_set_retention_applies_at_the_next_daily_run`; `src/events/upkeep/tests.rs` `db_004_the_daily_run_is_the_next_ten_past_midnight_utc`, `db_004_a_job_runs_at_start_retries_after_failures_then_runs_daily`; client `lib/retention/section.svelte.test.ts` "HUB-005: the note that changes apply at the next daily run is shown". | Test run result | open |
| 20 | A domain expiry check reads RDAP and reports the days left. Degraded below its warning days, Down after expiry. Its connections obey P3-003. | Section 6.4, CHK-009, D-29 | Hub `src/checks/probe/rdap/tests/observe.rs` `d_29_the_probe_reads_the_expiry_of_the_domain_in_lower_case`, `p3_003_a_registry_at_a_refused_address_is_never_reached`; `src/checks/probe/rdap/tests/status.rs` `d_29_the_status_follows_the_days_left`; `src/checks/scheduler/tests/domain.rs` `d_29_an_expired_domain_check_is_down_at_once_and_next_due_after_the_interval`; `src/checks/probe/rdap/tests/egress.rs` `p3_003_a_redirect_to_a_denied_address_is_refused_with_no_connection`; `tests/it/checks/domain_expiry.rs` `d_29_domain_days_left_floors_and_goes_negative_after_expiry`. One live RDAP query: step P-12. Checked at e87571f8: each of the four `d_29_` names above exists and its test does what its name says; `d_29_the_probe_reads_the_expiry_of_the_domain_in_lower_case` also asserts the `Expiring` judgement and one registry request, which its name does not say. | Domain, days left shown, status, time of the live query | open |
| 21 | The hub finds GPUs at the first collection, every 24 hours and after a restart, and collects metrics for NVIDIA, AMD and Apple silicon. On an Apple silicon Mac the detail view shows GPU usage and shared GPU memory, and the card shows no VRAM bar. | MET-006 to MET-009, R-073, D-30 | Discovery: hub `src/metrics/collector/tests/discovery.rs` `met_006_the_first_collection_looks_and_looks_again_after_24_hours`, `met_006_a_boot_time_later_by_more_than_the_tolerance_is_a_restart`; `src/metrics/collector/tests/gpus/rounds.rs` `met_006_the_first_collection_finds_and_stores_the_gpus_with_their_samples`. Vendor readings: `agent-core/readings/tests/gpu.rs` `met_016_nvidia_smi_capture_gives_the_gpu_with_memory_in_bytes`, `met_016_amd_files_give_busy_and_vram_for_one_card`, `met_016_ioreg_gives_one_reading_per_accelerator_in_order`, `met_007_apple_has_no_vram_and_no_figure_it_was_not_given`. Shared memory (agent path, IOKit `In use system memory`): `agent-core/readings/tests/gpu.rs` `met_016_ioreg_gives_one_reading_per_accelerator_in_order` (reads the memory figure); agent `crates/agent/src/readers/gpu/apple.rs` `met_007_use_and_in_use_memory_become_gpu_percent_and_gpu_memory_bytes_never_vram`; hub `src/metrics/tests/gpu_series.rs` `r_092_d_6_gpu_memory_bytes_is_stored_and_read_back_per_gpu`. The SSH path (`ioreg`) gives GPU usage only (spec MET-006 table). No VRAM without own memory: hub `src/metrics/collector/tests/gpus.rs` `met_007_no_vram_for_intel_or_apple_and_no_zero_for_a_missing_value`; client `lib/metrics/bars/bars.test.ts` "MET-007: no VRAM bar without a GPU that has its own memory, and never "0 %"" (the card); `lib/metrics/overview-charts.test.ts` "MET-007: no VRAM chart for a GPU without its own memory, even when a series arrives", "MET-009: GPU charts are titled with index, name and what they show". Live Apple silicon Mac with the agent: step P-13. Client display of the shared memory number (G-11, fixed: the detail view now charts `GPU_MEMORY_BYTES`): `lib/metrics/overview-charts.test.ts` "D-30: a GPU with shared memory gets a bytes chart for it, and no VRAM chart", "D-30, MET-007: a GPU with its own memory gets no shared memory chart, even when that series arrives"; `routes/devices/[id]/overview/apple-gpu.test.ts` "MET-009, D-30: the detail view of an Apple silicon Mac shows GPU usage and the shared memory in use, in bytes", "MET-007, D-30: an Apple silicon Mac shows no VRAM chart and the card draws no VRAM bar". | Mac model, a detail-view screenshot (GPU usage and shared memory), the card with no VRAM bar, and the result for an NVIDIA or AMD device if one was used | open |
| 22 | No log holds a password, key, token, cookie, API key, notification secret, webhook URL or session content. (Share token and recording key are dropped by R-111 and R-113.) | LOG-007, SEC-009 | Existing per-site tests: hub `tests/it/api_keys/revoke.rs` `log_007_an_api_key_secret_never_reaches_the_process_log`; `tests/it/bearer/metrics.rs` `log_007_a_scrape_is_logged_by_its_route_never_its_token` (supporting only: it checks the route, not a secret value); `tests/it/web_client.rs` `log_007_a_web_client_request_is_logged_without_its_path` (supporting only: it checks the path); `tests/it/wiring/uri_log.rs` `log_007_a_token_in_the_url_never_reaches_the_process_log`; `tests/it/agent/secrecy.rs` `log_007_the_process_log_of_the_whole_flow_holds_no_code_or_token`; `tests/it/totp.rs` `log_007_the_secret_and_codes_reach_no_log_event_or_audit_entry`; `tests/it/devices/credential.rs` `log_007_a_stored_secret_is_in_no_log_no_response_no_bus_message_and_no_plaintext_column`; `src/notifications/dispatch/tests/secrets.rs` `log_007_no_secret_is_logged_by_a_send_that_succeeds`, `log_007_no_secret_is_logged_recorded_or_delivered_by_a_failing_send`. Whole hub in one log (T5, merged): `hub/tests/it/acceptance/canary.rs` `log_007_no_phase_3_secret_reaches_the_process_log` (35 secret kinds, one TRACE log on every thread including the blocking pool, the `signInBrowser` cookie and the `Set-Cookie` value included). Known limits: the egress success path is covered by `hub/src/notifications/dispatch/tests/secrets.rs` (the address policy refuses loopback to integration tests); session content and tunnel tokens are covered by `hub/tests/it/sessions/leak.rs` `log_007_the_ticket_token_is_never_stored_logged_or_in_an_error`. Live log: `deploy/acceptance/log-scan.ts` (T3, merged; tests `deploy/test/acceptance-log-scan.test.ts`; step P-16). | T5 test result; T3 output from P-16 (line count scanned, hits: 0) | open |
| 23 | Each motion role that Phase 3 uses (theme change, toast, menu, modal, session tab open) has a reduced-motion form. The audit runs in WebKit and Chromium on the desktop and web clients, both themes. No open "layout jump" or "wrong motion". Nothing moves or blurs over a live display. (The watch page and playback are dropped by R-111 and R-113. Session tab open stays.) | MOT-040, MOT-041, MOT-043, Section 5.8, MV-001, MAT-008 | Audit: `docs/research/motion/phase-3.md` (T6, merged; verdict "NOT MET as worded" before T7a). T7a then fixed M-1 (the API key dialog lost 272.5 px in one frame) and W-1 (the last session tab's exit was hidden), measured by the T7a interface review in WebKit and Chromium: the surface holds 566.19 px and `zeroRectFrames` is 0. The audit file is not yet re-run; the acceptance run re-runs it. Fix tests: client `lib/api-keys/secret.svelte.test.ts` "MOT-007 / MV-002: the secret view keeps the form's height, so the surface does not drop in one frame"; `lib/sessions/strip/presence.test.ts` "MOT-005 / MV-002: the last tab closes with the strip still shown and out of the flow, and the strip goes when the exit ends" and "MOT-005: with two tabs closed in quick succession the strip stays shown until the second exit ends". The jsdom tests assert structure only; the browser measurement is the proof. No toast component exists in Phase 3 (T6 finding), so the toast role has nothing to measure; the update banner uses `fade` and is desktop only. W-2, S-1 (97 px, just under the line) and C-1 to C-3 are coulds on issue #1 (the WP-34 D-5 clause: a jump under 100 px is a could; main's ruling 2026-11-11). Unit tests: client `lib/motion/roles/transitions.test.ts` "MOT-040/041: %s is a fade only in both directions", "MOT-041: %s fades in over the reduced duration-base"; `lib/motion/options.test.ts` "MOT-041: under reduced motion durations come from the reduced table"; `lib/theme/motion-row.svelte.test.ts` "MOT-041, MOT-043: On makes the JS springs instant while the OS has no preference, with no reload"; `lib/sessions/strip/presence.test.ts` "MOT-040: under reduced motion the tab fades in place over duration-base"; `lib/sessions/files/transfer.test.ts` "MAT-008: no blur, translucency, filter or overlay position in the panel stylesheet". | The audit file with its measured table (`docs/research/motion/phase-3.md`, re-run after T7a) and no open layout jump or wrong motion | open |
| 24 | Gates pass 12/12, baseline empty, K1 has no `@pending`, Q2-05 to Q2-16 closed or on issue #1 with a reason. | CQ-010, CON-002, CQ-050 | `@pending` and baseline: `quality/acceptance/phase-3.ts` (T2, merged) with its tests in `quality/tests/acceptance.test.ts`. Known limit: an escaped `\"""` inside a GraphQL block string is not handled by the string blanking (T2 review could). Contract tests: hub `tests/it/contract/tests.rs` `con_002_the_pending_directive_definition_is_declared_by_k1_only`, `con_002_the_hub_schema_and_k1_declare_the_same_types_with_the_same_members`. Gates: `bun run gates` on the merged `phase-3` (R-101 environment). | Gate output 12/12 with the date and tip SHA; T2 output (`quality/acceptance/phase-3.ts`); the Q2-05 to Q2-16 list with each status | open |

## Operator steps

One ordered checklist. Run order: P-1 to P-15, P-17, then P-16 (the log scan runs last so it covers every step). Do setup once, then each live part. Record results in [operator-changes.md](operator-changes.md) or in the "Evidence" column of the matrix above.

Rules for the whole list:
- The operator types every real secret (SMTP password, ntfy token, webhook URL, Apprise URLs, API keys) into Settings personally. Agents never type, read or ask for a real secret.
- Agents run the scripts only against synthetic servers or a local hub with synthetic config.
- The scripts read any real key only from a file path the operator gives (`--key-file`). They never print it, and their output holds line numbers and kinds, not values.
- The operator may stop at any step. A step that fails is recorded as it is; it is not retried silently.

### P-1. Confirm the deployed image and compose

- Hardware: any machine on the VPN with a shell on the VPS (or the compose file open).
- Do:
  1. Confirm the running hub image is the one that holds the web client (the compose service points at the image built with the bun stage).
  2. Confirm the compose variables `ANIMA_WEB_ROOT=/usr/share/anima/web` and `ANIMA_PUBLIC_ORIGIN=https://hub.anima.ing` are set. Confirm there is no `recordings` volume. `CLIENT_ORIGINS` is a hub constant (`crates/hub/src/lib.rs`), not a compose variable: confirm the deployed hub is built from the tip that holds the two Tauri origins, or let the T4 CORS check show it.
  3. Confirm `ANIMA_PROBE_DENY` includes the hub's own VPN address.
- Look for: the three values as written above, and the hub container running with the root file system read-only.
- Record: the image tag or digest, and the date. Any VPS change needs the operator's confirmation in chat first (VPS-005).

### P-2. Run the T4 probe (no key)

- Hardware: a VPN machine with `bun`.
- Do:
  1. From the repository root, run `bun deploy/acceptance/probe.ts https://hub.anima.ing`.
  2. TLS is always verified, even when `NODE_TLS_REJECT_UNAUTHORIZED=0` is set. For a local hub with its own certificate, add `--ca <pem file>`.
  3. Plain `http://` is accepted only for a loopback host.
  4. Unset `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY` (either case) first. The probe refuses to run with any of them set (exit 2), so no request goes through a proxy.
  5. Read each line: `ok <check>` or `FAIL <check>: <reason>`, then `<n> checks, <m> failed`. A reason is fixed text: a status code, a header name or a K7 error code. The probe never prints a header value, a cookie or a body.
  6. Read the exit code. 0 means every check passed. 1 means a check failed. 2 means bad usage, a proxy variable, no answer from the hub, or a run longer than 120 s.
- Look for 11 `ok` lines:
  - `health`.
  - `index-csp`: the K7 CSP on `/`.
  - `index-headers`: the other K7 headers on `/`.
  - `index-cache`: `no-cache` on `/`.
  - `asset-cache`: `immutable` on one hashed file under `/_app/immutable/`.
  - `asset-headers`: the K7 headers on that file.
  - `graphql-foreign-cookie`: a `POST /graphql` from `https://evil.example` with a synthetic `__Host-anima` cookie is signed out. The answer is UNAUTHENTICATED, 401 or 403. It grants no CORS permission. It sets no cookie.
  - `socket-foreign-cookie`: the same socket opens, then the hub closes it with 4403 at `connection_init`. A socket that never opens is a FAIL.
  - `cors-client-origins`: both `CLIENT_ORIGINS` are allowed.
  - `cors-foreign-origin`: `https://evil.example` is not allowed.
  - `metrics-without-token`: 401.
- A synthetic cookie cannot sign anyone in. So `graphql-foreign-cookie` and `socket-foreign-cookie` show the signed-out answer only. P-3 and P-4 cover a real cookie.
- Record: the full output (it holds no secret), the date, criterion 4 evidence.

### P-3. Browsers and sign-in on macOS

- Hardware: a Mac on the VPN with Safari and Chrome.
- Do:
  1. In Safari, open `https://hub.anima.ing/`. Sign in with user name, password and TOTP code.
  2. Open the developer tools, then storage. Read the cookie row and the local and session storage.
  3. Repeat in Chrome.
- Look for: one cookie named `__Host-anima` with `HttpOnly`, `Secure` and `SameSite=Strict`; no token in local or session storage; the device list loads after sign in.
- Record: browser and version, result, the cookie attributes. Criteria 1 and 4.

### P-4. Browser on Linux, and the desktop app

- Hardware: a Linux machine with Firefox or Chromium; the desktop app on any machine.
- Do:
  1. Repeat P-3 in Firefox or Chromium.
  2. In the desktop app, sign in to the same hub, then sign out and sign in again.
  3. In a browser tab, press Sign out. Reload `https://hub.anima.ing/`.
- Look for: sign-in works in both; the desktop app still signs in; after Sign out the page asks for sign-in and the cookie is gone.
- Record: results per client. Criteria 1 and 4.

### P-5. Web client device list and detail view

- Hardware: any browser signed in (P-3).
- Do:
  1. Open the device list. Open one device. Read its live status.
  2. Open the Logs panel for the device.
  3. Check that Settings shows no Updates, Log folder or Builds rows, and the device page has no pop-out, web view tab, This Mac or System Settings control.
  4. Open the Web button of a device with a web interface.
- Look for: list, detail view, live status and logs panel work; the desktop-only controls are absent; the Web button opens a new tab straight to the device, with the pinned fingerprint beside it.
- Record: a screenshot of the list and the detail view, and the controls checked as absent. Criteria 2 and 3.

### P-6. SSH and VNC sessions in the web client

- Hardware: a Linux or macOS device with SSH on, and a device with VNC (a Mac with Screen Sharing on, or a Linux desktop).
- Do:
  1. Start an SSH session from the browser. Type one harmless command. End the session.
  2. Start a VNC session. Move the pointer, type a key. End the session.
- Look for: both sessions open, take input and end; the session tab opens without a layout jump.
- Record: device, time, result per session. Criterion 2.

### P-7. Enter the notification channels

- Hardware: any signed-in client (the desktop app or a browser).
- Do:
  1. In Settings, add one channel of each kind: webhook (for example Discord), ntfy (topic and token), email (SMTP host, port, user, password) and Apprise (the Apprise URLs). Type each real secret yourself.
  2. Mark at least one channel "on by default".
  3. Open the channel list and check that each target shows as an origin or host only.
- Look for: each channel saves; no secret appears after saving; the target shows with no path or credential.
- Record: the kinds added (never the secrets). Criteria 5, 6, 7, 11.

### P-8. Select channels on a check

- Hardware: a signed-in client.
- Do:
  1. Create a check (a TCP or HTTP check against a service you can stop). Confirm the on-by-default channels are pre-selected.
  2. Set a resend interval, for example 3.
- Look for: the channels listed on the form match the defaults. Criterion 6.
- Record: the check name and the channels selected.

### P-9. Send test on each channel

- Hardware: a signed-in client; the receiving app for each channel (Discord, the ntfy app, a mailbox, the Apprise target).
- Do:
  1. Press Send test on each channel. Time it.
  2. Check that the message arrives.
- Look for: a delivery result within 8 seconds for each channel; the message at the receiver.
- Record: per channel, the result and the seconds. Criterion 7.

### P-10. Down and Up cycle

- Hardware: the check from P-8 and a way to stop and start the service it watches.
- Do:
  1. Stop the service. Wait for the check to go Down.
  2. Wait for the resend interval to pass once.
  3. Start the service. Wait for the check to come Up.
- Look for: one Down message, one "still Down" message per N Down beats, and one Up message, on every selected channel.
- Record: per channel, the messages received (time, text kind). Criteria 5 and 8.

### P-11. Maintenance

- Hardware: the check from P-8.
- Do:
  1. Put the check in maintenance.
  2. Stop the service. Wait two intervals.
  3. Start the service. End maintenance.
- Look for: no notification during maintenance; no Up message after maintenance unless a Down message was sent first.
- Record: "none received" per channel, and the time window. Criterion 5.

### P-12. Live RDAP check

- Hardware: a signed-in client.
- Do:
  1. Add a domain expiry check for one real domain you own. Run it.
  2. Read the days left and the status.
- Look for: a date and a days-left value; status Up, or Degraded if below the warning days.
- Record: the domain, days left, status, time. Criterion 20.

### P-13. GPU metrics on the Apple silicon Mac

- Hardware: the operator's Apple silicon Mac with the push agent installed and running (the agent reads the shared memory figure; the SSH collector gives GPU usage only). The Mac is also a device with SSH on. An NVIDIA or AMD device is optional.
- Do:
  1. Wait for the first collection and the first agent push, or re-save the device to force a look.
  2. Open the device detail view and the device card.
  3. Optional: stop the agent and check that usage still shows from SSH and the shared memory number goes away.
- Look for: GPU usage in the detail view; the shared memory the GPU uses (agent only); no VRAM bar on the card.
- Record: the Mac model, the agent version, screenshots of both views, and the same for any NVIDIA or AMD device. Criterion 21. If the shared memory number does not show, record it as a failure of G-11.

### P-14. Optional: a real Prometheus scrape

- Hardware: a Prometheus server on a machine on the VPN, with a Headscale grant to `tag:hub` tcp:443 (the grant is a Headscale change; ask first).
- Do:
  1. Create a READ API key in Settings. Save the secret to a file with mode 600 on the Prometheus machine.
  2. Point `authorization.credentials_file` at that file and add the target `https://hub.anima.ing/metrics`.
  3. Unset `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY` (either case). The probe refuses to run with any of them set (exit 2), so the key never goes to a proxy.
  4. Run the T4 probe with the key: `bun deploy/acceptance/probe.ts https://hub.anima.ing --key-file <path>`.
  5. The file holds only the key (`anima_` and 43 characters). The probe refuses any other content and any file larger than 4 KiB. It warns when the file is readable by group or others. It never prints the key.
  6. Every document the probe sends with the key is invalid: an argument no field has, or a check ID no hub holds. So nothing changes even if a refusal failed.
- Look for:
  - The target is `up`.
  - The P-2 lines, all `ok`.
  - `metrics-with-key`: 200 with the Prometheus text content type.
  - `metrics-text`: the strict Prometheus reader accepts the text.
  - `metrics-families`: the seven K7 families, all gauges, no other.
  - `metrics-labels`: only the D-24 label names. `window` is only `1d`, `7d` or `30d`.
  - `metrics-label-values`: IDs are UUIDs. No value looks like a time, a date, a duration, a number, an address or a status. This value check is a heuristic. The exact D-24 label names in `metrics-labels` are the hard rule. A flagged value that is a stable name the operator typed (for example `host:port`) is a false fail; compare it by eye.
  - `key-accepted`: the hub knows the key.
  - `read-key-mutation`: FORBIDDEN for an ordinary mutation.
  - One `refused-root-<root>` per root of D-23 and R-100 (30 roots, with the five VPN roots the VPN work added): FORBIDDEN by the key-kind rule, not only as a read.
  - With a WRITE key, `read-key-mutation` fails and says so. Use a READ key for criterion 13.
- Record: the target state, one query result, the T4 output. Criteria 12 and 13. Revoke the key after the run and note the revoke.

### P-15. Optional: API key checks in Settings

- Hardware: a signed-in client.
- Do: create a key with a scope and an expiry date; note that the secret shows once; revoke it and use it once more.
- Look for: the use after revoke is refused; the audit list holds the create and the revoke.
- Record: result. Criterion 13.

### P-17. Desktop app: the update banner

- Hardware: a desktop app build (macOS or Linux) with an older version installed, and a newer version available from the release channel. Desktop only; the web client never loads the updater (UPD-001).
- Do:
  1. Open the older desktop app. Let it check for updates and install the newer version (UPD-004; main's ruling 2026-11-11).
  2. Read the banner. Check its role in the developer tools or the accessibility tree.
  3. Press "Restart to update".
- Look for:
  - A banner with the text "Anima <version> is installed" and a "Restart to update" button.
  - The banner has `role="status"` (spec UPD-004, line 252 of `docs/design-spec.md`).
  - Restart applies the update: the app starts at the new version.
- Record: the old and the new version, and a screenshot of the banner. Criterion 2.
- Order: this step is listed above P-16 but P-16 stays the last to run, so the log scan covers it.

### P-16. Last: the T3 log scan over the live hub log

- Hardware: a shell on the VPS (or the hub log copied to a VPN machine), after every other step (P-17 is listed above it and runs before it).
- Do:
  1. Count the log: `docker compose logs --no-color hub | wc -l`.
  2. Note the span of the log: `docker compose logs --no-color --timestamps hub | sed -n '1p;$p' | awk '{print $1, $2, $3}'`. Each line starts with the service, a `|` and the timestamp (`hub-1  | 2026-11-11T09:00:00.000000000Z ...`), and awk prints only those three fields, so no log text shows. Check that the first output line has this layout (service, `|`, timestamp) before you record it. If it does not, stop and record the layout you see, without the log text.
  3. Run the scan with pipefail, so a failed `docker compose` is not read as a clean empty log: `bash -o pipefail -c 'docker compose logs --no-color hub | bun deploy/acceptance/log-scan.ts'`. To add known values (for example the test channel names and URLs), add `--needles <file>`; the file stays on the machine.
  4. Read the result. The script prints only a line number and a kind for each hit, then `scanned <n> lines, <m> hits`. An empty log exits 2.
- Look for:
  - Exit code 0.
  - "0 hits".
  - A scanned line count that is equal to or more than the count from step 1. The hub keeps logging, so a few more lines is correct.
  - A first timestamp from before step P-1. This shows that the log covers every step.
- Record: the date, the log span (first and last timestamps), the number of lines scanned against the count from step 1, the hit count. If there is a hit, record the line numbers and kinds and open a security finding; do not paste the log. Criterion 22.

## Gaps

Each criterion part that no existing test and no script proved when this list was written. Closed items say by which task.

- **G-1 (criterion 2, logs panel and live data).** The logs panel and live status are tested only through the shared device page tests, not in "browser answers" mode. Proposal: a client test that mounts the device page with the browser answers and checks the logs panel, the status pill and that no desktop control renders. Live step P-5 covers acceptance. Closed for what the device page itself shows: `routes/devices/[id]/detail/browser-page.test.ts` mounts it with the browser answers and proves the logs panel, the live status pill, no control named for a desktop feature (text scan) and, with a desktop-answers positive control, that the Web button opens a browser tab and no web view tab. The other desktop-only controls are not on the device page; their tests are in row 2.
- **G-2 (criterion 7, 8 s limit for ntfy, email and Apprise).** `not_004_a_test_to_a_silent_target_ends_within_the_time_limit` proves the limit for a webhook only. Closed by WP-53 T7b: `not_004_a_test_to_a_silent_ntfy_server_ends_within_the_time_limit`, `not_004_a_test_to_a_silent_mail_server_ends_within_the_time_limit` and `not_004_a_test_to_a_silent_apprise_server_ends_within_the_time_limit` in `hub/src/notifications/graphql/tests/test_send.rs`, plus `not_004_the_send_time_limit_is_8_seconds`, which pins `TIME_LIMIT` to 8 s, each against a loopback listener that accepts and never answers. Live step P-9 covers the real channels.
- **G-3 (criterion 9, domain expiry notices).** No test drives a domain expiry check beat to a queued expiry notice at 21, 14 and 7 days. The scheduler tests prove none under maintenance and none for a renamed domain; the decision tests are pure. Closed by WP-53 T7b in `hub/src/checks/scheduler/tests/domain.rs`: `not_006_a_domain_check_beat_queues_one_notice_for_each_threshold_it_crosses` beats a check to 20 days left (one `domain_expiry` row), an hour later (no second row), and a week on at 13 days (a second row); `not_006_a_domain_first_seen_at_six_days_queues_one_notice` covers 6 days. The 7-day threshold itself is covered by the pure decision tests `not_006_one_notification_per_threshold_as_days_run_down` and `not_006_a_threshold_is_crossed_only_when_fewer_whole_days_are_left`. The beats cannot run further ahead than 7 days because the delivery partitions reach only 7 days past the database clock, so 6 days is covered by first sight.
- **G-4 (criterion 4, the literal `CLIENT_ORIGINS` list).** No test pins the list: `hub/tests/it/graphql.rs` iterates the constant. Proposal: one test that asserts the list equals `tauri://localhost` and `http://tauri.localhost`; T4 checks it on the live hub. Closed by WP-53 T4: `deploy/test/acceptance-probe-verdicts.test.ts` "SEC-002 G-4: the client origins are CLIENT_ORIGINS of lib.rs" reads `CLIENT_ORIGINS` from `hub/src/lib.rs` and requires it to equal the probe's list, `tauri://localhost` and `http://tauri.localhost`. The `cors-client-origins` check of `deploy/acceptance/probe.ts` shows both on the live hub (step P-2).
- **G-5 (criterion 13, audit per key use).** No spec text asks for an audit entry on each key use. Proposal: read "each step" as create and revoke (covered by `log_005_create_and_revoke_each_leave_an_audit_entry_and_an_event`) and say so in the criterion at acceptance.
- **G-6 (criterion 13, root list after R-113).** Done by WP-48 T10 (`75f8b6cd`). `openRecording` and `deleteRecording` are gone from `REFUSED` in `hub/tests/it/api_keys/refused.rs` and from `API_KEY_REFUSED_ROOTS` in `hub/src/access/guard.rs`. The VPN work then added `createVpnInvite`, `createRouterKey`, `registerVpnNode`, `renewVpnKey` and `approveSiteRoute` to both (VP-7, VP-10). Each list now holds 30 roots. `startSession`, `openFiles` and `signInBrowser` are in both. The T4 probe sends the same 30 roots.
- **G-7 (criteria 1 and 4, real browsers).** No test proves `HttpOnly`, no script-readable token and the cross-origin refusal in a real browser. Proposal: operator steps P-3 and P-4; optionally a Playwright run in WebKit and Chromium against a local hub with synthetic config.
- **G-8 (criterion 5, real endpoints and a real Apprise service).** The tests use synthetic receivers. Proposal: operator steps P-7 to P-11.
- **G-9 (criterion 12, Prometheus text on the live hub).** The in-process tests do not prove the deployed text. Proposal: T4 with `--key-file`, and the optional step P-14. T4 is merged (`deploy/acceptance/probe.ts`); the run against the live hub is step P-14.
- **G-10 (criterion 19, retention path).** `hub_005_each_domain_drops_by_its_own_period` (`hub/src/upkeep/tests.rs`) already shows a changed period applies at the next run. Closed by WP-53 T7b: `hub_005_a_period_set_through_set_retention_applies_at_the_next_daily_run` sends `setRetention` for events (30 days), shows nothing is dropped at once, runs the daily job and shows the 40-day-old row gone and the 20-day-old row kept. The daily scheduling is proved with an injected clock by `db_004_the_daily_run_is_the_next_ten_past_midnight_utc` and `db_004_a_job_runs_at_start_retries_after_failures_then_runs_daily` in `hub/src/events/upkeep/tests.rs`.
- **G-11 (criterion 21, shared memory in the client).** No client code or test shows the GPU shared memory number: the client sources outside the generated types do not read the `gpu_memory_bytes` series, and `bars.test.ts` proves only that the VRAM bar is absent. Hub storage and the agent reading are proved (see row 21). Proposal: a client test and, if the display is missing, a client fix for the detail view; live step P-13 shows it. Closed: the client did not read the `GPU_MEMORY_BYTES` series; `lib/metrics/overview-charts.ts` now draws a "GPU <index> <name> memory in use (shared)" chart in IEC bytes beside the usage chart (D-30, MET-009), drawn only for a GPU without its own memory (hasVram false); tests `overview-charts.test.ts` "D-30: a GPU with shared memory gets a bytes chart for it, and no VRAM chart", "D-30, MET-007: a GPU with its own memory gets no shared memory chart, even when that series arrives", "D-30: two GPUs without their own memory each get a chart from their own shared memory series", `live.test.ts` "D-30: a live GPU_MEMORY_BYTES sample labelled \"0\" joins the shared memory series of GPU 0, and no other series" and `overview/apple-gpu.test.ts` (shared memory shown in GiB; no VRAM chart or bar). Live step P-13 still shows it on the Mac. Known spec gap: spec line 1010 asks for the share as a number; the value is shown as a chart (aria-label and axis). The visible number waits for the shared chart header change in the UI redesign plan (main's ruling, 2026-11-12).
- **G-12 (criterion 21, Apple silicon on real hardware).** The tests use captured output. Proposal: step P-13 with the agent.
- **G-13 (criterion 22, one log across all Phase 3 secrets).** The per-site `log_007` tests do not scan one log for every secret. Proposal: T5 (hub canary) and T3 (live scan). Closed by WP-53 T5: `hub/tests/it/acceptance/canary.rs` `log_007_no_phase_3_secret_reaches_the_process_log` (35 secret kinds, TRACE on every thread including the blocking pool). Known limits: the egress success path is covered by `hub/src/notifications/dispatch/tests/secrets.rs` (loopback is refused to integration tests); session content and tunnel tokens are covered by `hub/tests/it/sessions/leak.rs` `log_007_the_ticket_token_is_never_stored_logged_or_in_an_error`. The live log part stays step P-16 (T3 `deploy/acceptance/log-scan.ts`).
- **G-14 (criterion 22, the `signInBrowser` cookie value in logs).** No `log_007` test for the browser cookie or the `Set-Cookie` header. Proposal: include it in T5. Closed by WP-53 T5: the same canary test sends `signInBrowser` and searches the log for the cookie value and the `Set-Cookie` header. The live log part stays step P-16.
- **G-15 (criterion 23, motion audit).** No audit file exists for Phase 3, and the unit tests do not measure layout jumps in WebKit and Chromium. Proposal: T6 now for theme, toast, menu and modal; session tab open after WP-48 T10. Closed for the measured roles: T6 wrote `docs/research/motion/phase-3.md` (verdict "NOT MET as worded"), and T7a fixed M-1 and W-1 (`lib/api-keys/secret.svelte.test.ts` "MOT-007 / MV-002: the secret view keeps the form's height, so the surface does not drop in one frame"; `lib/sessions/strip/presence.test.ts` "MOT-005 / MV-002: the last tab closes with the strip still shown and out of the flow, and the strip goes when the exit ends"), measured by the T7a interface review in WebKit and Chromium (the surface holds 566.19 px; `zeroRectFrames` 0). The audit file still reads "NOT MET as worded" until the acceptance run re-runs it. W-2, S-1 and C-1 to C-3 are coulds on issue #1 (WP-34 D-5 clause: under 100 px is a could; main's ruling 2026-11-11). No toast component exists in Phase 3 (T6 finding).
- **G-16 (criterion 24, Q2-05 to Q2-16).** No script reads the status of the Phase 2 questions. Proposal: main lists each as closed or on issue #1 in this file at acceptance. `quality/acceptance/phase-3.ts` (T2, merged; tests `quality/tests/acceptance.test.ts`) now checks the `@pending` and baseline parts. Known limit: an escaped `\"""` inside a GraphQL block string is not handled by the string blanking (T2 review could).

## Issue #1 list (coulds and known gaps from WP-53)

- T6 motion, `apps/client/src/lib/motion/roles/transitions.ts` lines 87 to 95: W-2, the menu exit scales from the panel centre, not from the control (at most 2.4 px).
- T6 motion, Settings API keys skeleton: S-1, the skeleton is 97 px short of the loaded list.
- T6 motion: C-1 and C-2, colour transitions during a theme change.
- T6 motion, `apps/client/src/lib/sessions/screen/screen.ts`: C-3, the SSH first frame stalls.
- T7a, `apps/client/src/lib/api-keys/secret.svelte.test.ts` and `apps/client/src/lib/sessions/strip/presence.test.ts`: the strip and dialog unit tests assert structure (jsdom has no layout); the browser measurement is the proof.
- T7a, `apps/client/src/lib/sessions/strip/SessionTabs.svelte`: the `.leaving` margin and the ghost tab overlap the search field for about 170 ms.
- T7c, `apps/client/src/lib/metrics/overview-charts.ts`: the visible shared-memory number (spec line 1010) waits for the shared chart header change in the UI redesign plan (main's ruling 2026-11-12); also recorded under G-11 as a known spec gap.
- T7c, `apps/client/src/routes/devices/[id]/detail/browser-page.test.ts`: the `BROWSER` default in `beforeEach` is not exercised by every test.
- T3 log-scan, `deploy/acceptance/log-scan.ts`: view cost is linear per view; the needle spellings (`+`, `%2F`, base64) are not searched; lower-case compound field names; blank lines in the test file.
- T4 probe, `deploy/acceptance/probe.ts`: the VOLATILE label patterns can falsely fail a stable operator name (`host:port`, a date); the 120 s run deadline has no test.
- T5 canary, `hub/tests/it/acceptance/canary.rs`: the 6-digit TOTP search has a small false-hit chance that cannot be reproduced; fragments under 12 characters are not searched.
- T8 deploy tests, `deploy/test/secrets-token.test.ts`: the mode check no longer proves a regular file (add `isFile` or `lstatSync`).
- T8 deploy tests, `deploy/headscale/tests`, `deploy/dns/tests` and `deploy/headscale/bin/broker-build.ts`: still use `Bun.spawnSync` (the gate-7 hang pattern); the Bun upgrade is tracked by main.
- T9 and T9b, `hub/src/builds/jobs.rs` (report around lines 198 to 215, uploaded around 279 to 295, claim from line 134) and `hub/src/notifications/dispatch/claimed.rs`: other error paths drop a transaction that holds a lock relevant to SKIP LOCKED; all low.
- T9b, `hub/src/builds/publishing/tests/holds.rs`: depends on the default current-thread test runtime (add a note or an assert).
- T9, `hub/src/builds/publishing/tries.rs`: no test for the Retry arm of `record`.
- Handoff to main: a hub test that pins async-graphql validation (ValidationMode Strict, unknown argument names refused), raised by the T4 security review; it touches the schema wiring (a programme file).
- Deploy suite hang: the root cause is Bun 1.3.11 `spawnSync` busy-waiting on a zombie child; T8 works around it. `log_005` is fixed by T10 (T10, in review).
