# UI component inventory, 2026-11-11

Phase 0 of the UI redesign (`docs/ui-redesign-plan.md`). Input to `docs/ui-critique-2026-11-11.md`.
Measured on branch `ui-p0-gallery` (head of `ui-redesign` plus the gallery). Script: `inventory.py` (scratchpad, not committed); regex-based, so counts are indicative, not exact.

## How to read this

- **Users**: number of files that import the component (`.svelte` and `.ts`), tests and fixtures included; the number in brackets is the count without tests and fixtures. `route` marks a SvelteKit `+page`/`+layout` file, which the router mounts.
- **Bypass**: a literal in a `<style>` block or inline `style` that a token should carry: raw colour (hex, rgb, hsl, oklch), a length (`px`, `rem`, `em`, `ch`, `vh`, `vw`; zero excluded) in any property, a literal `z-index`, a `font-size` or `line-height` number, a border style written out instead of a `--border-*` token, a duration in `transition`/`animation`, and a raw breakpoint in `@media` or `@container`. Values inside `var(...)` are tokens and are not counted.
- **Var** is the number of `var(--...)` uses in the component's styles; **calc** counts `calc`, `min`, `max`, `clamp` uses (arithmetic on tokens, a sign that the token scale lacks a step).

## Headline

- 166 components (138 under `src/lib`, 28 under `src/routes`).
- 140 of them have no literal at all. 25 style literals in 17 components, plus 16 raw breakpoints.
- 2686 token references against 25 literals: the colour, type, border and layer literals are already kept out by lint. The bypass that is left is in sizes and breakpoints (see the summary), and in 155 `calc`/`min`/`max`/`clamp` expressions that build a value the token scale does not offer.

## Components by folder

### `lib/access`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/access/SignOutRow.svelte` | 2 (1) | 22 | 1 | - |

### `lib/api-keys`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/api-keys/ApiKeysSection.svelte` | 4 (1) | 11 | 0 | - |
| `lib/api-keys/KeyRow.svelte` | 1 (1) | 20 | 0 | - |
| `lib/api-keys/KeySecret.svelte` | 2 (2) | 29 | 2 | raw breakpoint x1 |
| `lib/api-keys/NewKeyDialog.svelte` | 1 (1) | 33 | 0 | - |

### `lib/builds`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/builds/BuildsRow.svelte` | 2 (1) | 37 | 1 | - |

### `lib/charts`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/charts/ChartFrame.svelte` | 2 (2) | 29 | 5 | - |
| `lib/charts/ChartSkeleton.svelte` | 2 (1) | 0 | 0 | - |
| `lib/charts/LineChart.svelte` | 3 (2) | 8 | 0 | - |

### `lib/checks`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/checks/HeartbeatBar.svelte` | 4 (3) | 15 | 1 | - |
| `lib/checks/defaults/DefaultsSection.svelte` | 2 (1) | 12 | 1 | - |
| `lib/checks/form/CheckForm.svelte` | 2 (1) | 13 | 1 | - |
| `lib/checks/form/channels/ChannelsSection.svelte` | 1 (1) | 40 | 1 | - |
| `lib/checks/form/fields/AdvancedFields.svelte` | 1 (1) | 0 | 0 | - |
| `lib/checks/form/fields/GroupFields.svelte` | 1 (1) | 13 | 0 | - |
| `lib/checks/form/fields/HttpFields.svelte` | 1 (1) | 11 | 0 | - |
| `lib/checks/form/fields/PressButton.svelte` | 2 (2) | 20 | 0 | - |
| `lib/checks/form/fields/SelectField.svelte` | 4 (4) | 19 | 0 | - |
| `lib/checks/form/fields/TextField.svelte` | 7 (7) | 18 | 0 | - |
| `lib/checks/form/fields/ToggleField.svelte` | 2 (2) | 9 | 0 | - |
| `lib/checks/form/fields/TypeFields.svelte` | 1 (1) | 2 | 0 | - |
| `lib/checks/form/probes/DomainExpiryFields.svelte` | 1 (1) | 3 | 1 | - |
| `lib/checks/form/probes/ProbeFields.svelte` | 1 (1) | 0 | 0 | - |
| `lib/checks/form/probes/SnmpFields.svelte` | 1 (1) | 0 | 0 | - |

### `lib/devices`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/devices/actions/DeviceActions.svelte` | 2 (2) | 11 | 1 | - |
| `lib/devices/actions/WarningBadge.svelte` | 3 (3) | 7 | 0 | raw length (size (no token family)) x2; raw breakpoint x1 |
| `lib/devices/actions/WebMenu.svelte` | 3 (2) | 29 | 3 | raw length (size (no token family)) x1 |
| `lib/devices/add/AddDeviceModal.svelte` | 3 (1) | 0 | 0 | - |
| `lib/devices/add/MachineChoice.svelte` | 1 (1) | 18 | 2 | - |
| `lib/devices/card/DeviceCard.svelte` | 4 (1) | 32 | 0 | - |
| `lib/devices/card/DeviceGrid.svelte` | 1 (1) | 2 | 1 | - |
| `lib/devices/card/DeviceTable.svelte` | 4 (1) | 24 | 0 | - |
| `lib/devices/card/StatusEdge.svelte` | 3 (2) | 7 | 0 | - |
| `lib/devices/card/StatusPill.svelte` | 4 (3) | 25 | 2 | - |
| `lib/devices/form/form/DeviceForm.svelte` | 3 (2) | 8 | 0 | - |
| `lib/devices/form/form/Field.svelte` | 5 (5) | 24 | 0 | - |
| `lib/devices/form/form/GeneralSection.svelte` | 1 (1) | 2 | 0 | raw breakpoint x1 |
| `lib/devices/form/form/RemoteLoginButton.svelte` | 2 (1) | 23 | 0 | - |
| `lib/devices/form/form/SshSection.svelte` | 1 (1) | 6 | 0 | - |
| `lib/devices/form/form/TagsField.svelte` | 1 (1) | 35 | 1 | - |
| `lib/devices/form/form/TestConnection.svelte` | 1 (1) | 4 | 0 | - |
| `lib/devices/form/policy/PolicyNotice.svelte` | 1 (1) | 3 | 0 | - |
| `lib/devices/form/policy/PolicySection.svelte` | 1 (1) | 24 | 0 | - |
| `lib/devices/form/remote/RdpSection.svelte` | 1 (1) | 15 | 0 | - |
| `lib/devices/form/remote/TransferSection.svelte` | 1 (1) | 1 | 0 | - |
| `lib/devices/form/remote/VncSection.svelte` | 1 (1) | 8 | 0 | - |
| `lib/devices/form/remote/test/RemoteTest.svelte` | 1 (1) | 10 | 0 | - |
| `lib/devices/form/web/Suggestions.svelte` | 1 (1) | 9 | 0 | - |
| `lib/devices/form/web/WebInterfaceRow.svelte` | 1 (1) | 25 | 0 | - |
| `lib/devices/form/web/WebInterfaces.svelte` | 3 (2) | 7 | 0 | - |
| `lib/devices/os/OsIcon.svelte` | 5 (5) | 1 | 0 | - |
| `lib/devices/screen-sharing/ScreenSharingButton.svelte` | 4 (2) | 19 | 3 | raw border style (border/outline) x1; raw length (size (no token family)) x3; raw breakpoint x1 |
| `lib/devices/screen/DeviceScreen.svelte` | 6 (1) | 24 | 1 | - |
| `lib/devices/screen/Skeleton.svelte` | 2 (1) | 11 | 1 | - |
| `lib/devices/toolbar/Menu.svelte` | 2 (2) | 40 | 4 | raw length (size (no token family)) x1 |
| `lib/devices/toolbar/Toolbar.svelte` | 2 (1) | 41 | 5 | - |

### `lib/events`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/events/panel/LogFilters.svelte` | 1 (1) | 15 | 1 | - |
| `lib/events/panel/LogList.svelte` | 3 (2) | 38 | 5 | - |
| `lib/events/panel/LogsPanel.svelte` | 3 (1) | 38 | 3 | - |

### `lib/forms`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/forms/FormButton.svelte` | 36 (35) | 31 | 0 | - |
| `lib/forms/disclosure/Disclosure.svelte` | 7 (3) | 22 | 0 | - |
| `lib/forms/modal/ConfirmModal.svelte` | 11 (7) | 3 | 0 | - |
| `lib/forms/modal/HostKeyModal.svelte` | 3 (1) | 12 | 0 | - |
| `lib/forms/modal/Modal.svelte` | 12 (11) | 50 | 8 | - |
| `lib/forms/modal/tests/FooterActions.svelte` | 1 (0) | 0 | 0 | - |
| `lib/forms/modal/tests/FooterHost.svelte` | 1 (0) | 0 | 0 | - |

### `lib/hub`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/hub/notice/LiveNotice.svelte` | 4 (3) | 6 | 0 | - |

### `lib/hub-address`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/hub-address/HubRows.svelte` | 1 (1) | 25 | 2 | - |

### `lib/icons`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/icons/Icon.svelte` | 47 (46) | 4 | 0 | raw length (type: line-height) x1 |

### `lib/metrics`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/metrics/agent/AgentPanel.svelte` | 2 (1) | 19 | 0 | - |
| `lib/metrics/agent/ThisComputerButton.svelte` | 1 (1) | 0 | 0 | - |
| `lib/metrics/bars/MetricBars.svelte` | 3 (2) | 25 | 1 | - |

### `lib/nav`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/nav/Nav.svelte` | 1 (1) | 74 | 3 | raw length (size (no token family)) x1 |
| `lib/nav/collapse/CollapseToggle.svelte` | 1 (1) | 15 | 0 | - |
| `lib/nav/collapse/ResizeEdge.svelte` | 1 (1) | 5 | 1 | - |

### `lib/notifications`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/notifications/Field.svelte` | 8 (8) | 26 | 1 | - |
| `lib/notifications/NotificationsSection.svelte` | 5 (1) | 8 | 1 | - |
| `lib/notifications/channels/ChannelList.svelte` | 1 (1) | 9 | 0 | - |
| `lib/notifications/channels/ChannelRow.svelte` | 1 (1) | 34 | 4 | raw breakpoint x1 |
| `lib/notifications/editor/ChannelEditor.svelte` | 1 (1) | 0 | 0 | - |
| `lib/notifications/editor/ChannelForm.svelte` | 1 (1) | 28 | 2 | - |
| `lib/notifications/editor/SecretField.svelte` | 4 (4) | 6 | 0 | - |
| `lib/notifications/editor/kinds/AppriseFields.svelte` | 1 (1) | 0 | 0 | - |
| `lib/notifications/editor/kinds/EmailFields.svelte` | 1 (1) | 0 | 0 | - |
| `lib/notifications/editor/kinds/NtfyFields.svelte` | 1 (1) | 0 | 0 | - |
| `lib/notifications/editor/kinds/WebhookFields.svelte` | 1 (1) | 21 | 2 | - |
| `lib/notifications/settings/SettingsForm.svelte` | 1 (1) | 18 | 3 | raw breakpoint x1 |
| `lib/notifications/template/TemplateFields.svelte` | 2 (2) | 0 | 0 | - |

### `lib/process-log`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/process-log/LogFolderRow.svelte` | 2 (1) | 14 | 0 | - |

### `lib/retention`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/retention/RetentionRow.svelte` | 1 (1) | 4 | 1 | - |
| `lib/retention/RetentionSection.svelte` | 2 (1) | 10 | 0 | - |

### `lib/sessions`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/sessions/files/FilesPanel.svelte` | 2 (1) | 55 | 7 | - |
| `lib/sessions/live/LiveSessions.svelte` | 4 (1) | 51 | 5 | raw length (size (no token family)) x1 |
| `lib/sessions/recording/asciicast/AsciicastPlayer.svelte` | 1 (0) | 3 | 0 | - |
| `lib/sessions/recording/asciicast/Controls.svelte` | 2 (1) | 21 | 0 | - |
| `lib/sessions/strip/SessionPanes.svelte` | 1 (1) | 1 | 0 | - |
| `lib/sessions/strip/SessionTab.svelte` | 1 (1) | 33 | 3 | - |
| `lib/sessions/strip/SessionTabs.svelte` | 1 (1) | 9 | 3 | - |
| `lib/sessions/strip/menu/TabMenu.svelte` | 2 (1) | 20 | 1 | - |
| `lib/sessions/view/EndedPanel.svelte` | 2 (2) | 22 | 0 | - |
| `lib/sessions/view/SessionView.svelte` | 3 (1) | 38 | 2 | - |
| `lib/sessions/view/certificate/CertificateModal.svelte` | 2 (1) | 32 | 0 | - |
| `lib/sessions/view/credentials/CredentialPrompt.svelte` | 2 (1) | 37 | 0 | - |
| `lib/sessions/view/filing/hubbed.fixture.svelte` | 1 (0) | 0 | 0 | - |
| `lib/sessions/view/hostkey/HostKeyModal.svelte` | 1 (1) | 33 | 0 | - |
| `lib/sessions/view/menu/KeysMenu.svelte` | 2 (2) | 40 | 5 | - |
| `lib/sessions/view/pane/FindBar.svelte` | 1 (1) | 24 | 3 | raw length (size (no token family)) x1 |
| `lib/sessions/view/pane/LinkConfirm.svelte` | 1 (1) | 21 | 1 | - |
| `lib/sessions/view/pane/Notice.svelte` | 2 (2) | 11 | 1 | - |
| `lib/sessions/view/recording/RecordingBadge.svelte` | 1 (0) | 18 | 0 | - |
| `lib/sessions/view/remote/RemoteToolbar.svelte` | 1 (1) | 17 | 1 | - |
| `lib/sessions/view/remote/RemoteView.svelte` | 2 (1) | 8 | 0 | - |
| `lib/sessions/view/steps/ConnectionSteps.svelte` | 2 (2) | 21 | 1 | - |
| `lib/sessions/wake/WakeButton.svelte` | 2 (1) | 20 | 1 | raw length (size (no token family)) x1 |

### `lib/settings`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/settings/SettingRow.svelte` | 10 (10) | 40 | 4 | raw breakpoint x1 |
| `lib/settings/SettingsSection.svelte` | 1 (1) | 18 | 3 | - |

### `lib/theme`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/theme/MotionRow.svelte` | 2 (1) | 0 | 0 | - |
| `lib/theme/ThemeRow.svelte` | 2 (1) | 0 | 0 | - |

### `lib/typeface`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/typeface/TypefaceRows.svelte` | 2 (1) | 0 | 0 | - |

### `lib/updates`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/updates/DesktopRows.svelte` | 1 (1) | 0 | 0 | - |
| `lib/updates/UpdateBanner.svelte` | 2 (1) | 33 | 1 | raw length (size (no token family)) x1 |
| `lib/updates/UpdatesRow.svelte` | 2 (1) | 21 | 2 | - |
| `lib/updates/tests/DesktopProbe.svelte` | 2 (0) | 0 | 0 | - |
| `lib/updates/tests/LogRowStub.svelte` | 0 (0) | 0 | 0 | - |
| `lib/updates/tests/RowStub.svelte` | 0 (0) | 0 | 0 | - |

### `lib/vpn`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/vpn/VpnSection.svelte` | 2 (1) | 15 | 0 | - |
| `lib/vpn/device/DeviceVpn.svelte` | 2 (1) | 7 | 0 | - |
| `lib/vpn/enrol/EnrolModal.svelte` | 2 (1) | 36 | 2 | raw length (other) x1; raw breakpoint x1 |
| `lib/vpn/keys/NodeKeys.svelte` | 2 (1) | 26 | 0 | raw breakpoint x1 |
| `lib/vpn/keys/RenewKey.svelte` | 1 (1) | 8 | 2 | raw length (other) x1; raw breakpoint x1 |
| `lib/vpn/sites/ApproveRoute.svelte` | 2 (2) | 9 | 2 | raw length (other) x1; raw breakpoint x1 |
| `lib/vpn/sites/SitePaths.svelte` | 2 (1) | 27 | 0 | raw breakpoint x1 |

### `lib/web-interfaces`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `lib/web-interfaces/open/PinnedFingerprint.svelte` | 2 (1) | 4 | 1 | - |
| `lib/web-interfaces/tab/BrowserPage.svelte` | 1 (1) | 30 | 2 | - |
| `lib/web-interfaces/tab/DownPage.svelte` | 1 (1) | 26 | 0 | - |
| `lib/web-interfaces/tab/Toolbar.svelte` | 1 (1) | 29 | 1 | raw length (size (no token family)) x4; raw breakpoint x1 |
| `lib/web-interfaces/tab/TrustPage.svelte` | 1 (1) | 50 | 2 | - |
| `lib/web-interfaces/tab/WebTab.svelte` | 5 (1) | 9 | 0 | - |

### `routes`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/+layout.svelte` | route | 12 | 1 | raw length (size (no token family)) x2 |
| `routes/+page.svelte` | route | 0 | 0 | - |

### `routes/devices`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/+page.svelte` | route | 4 | 0 | - |

### `routes/devices/[id]`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/+page.svelte` | route | 0 | 0 | - |

### `routes/devices/[id]/checks`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/checks/CheckEditor.svelte` | 2 (1) | 0 | 0 | - |
| `routes/devices/[id]/checks/CheckHistory.svelte` | 1 (1) | 27 | 4 | - |
| `routes/devices/[id]/checks/CheckRow.svelte` | 1 (1) | 26 | 0 | raw breakpoint x1 |
| `routes/devices/[id]/checks/ChecksPanel.svelte` | 1 (1) | 16 | 0 | - |
| `routes/devices/[id]/checks/ResponseChart.svelte` | 2 (1) | 0 | 0 | - |

### `routes/devices/[id]/detail`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/detail/DetailBody.svelte` | 1 (1) | 13 | 0 | - |
| `routes/devices/[id]/detail/DeviceMissing.svelte` | 2 (2) | 2 | 0 | - |

### `routes/devices/[id]/header`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/header/DetailChips.svelte` | 1 (1) | 11 | 0 | - |
| `routes/devices/[id]/header/Header.svelte` | 5 (2) | 51 | 6 | raw breakpoint x2 |
| `routes/devices/[id]/header/HeaderActions.svelte` | 1 (1) | 5 | 0 | raw border style (border/outline) x1 |

### `routes/devices/[id]/overview`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/overview/MetricsSection.svelte` | 1 (1) | 31 | 0 | - |
| `routes/devices/[id]/overview/MetricsSkeleton.svelte` | 2 (2) | 0 | 0 | - |
| `routes/devices/[id]/overview/OverviewPanel.svelte` | 3 (1) | 0 | 0 | - |
| `routes/devices/[id]/overview/OverviewSkeleton.svelte` | 2 (1) | 6 | 0 | - |

### `routes/devices/[id]/overview/facts`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/overview/facts/HostKeyCard.svelte` | 1 (1) | 14 | 0 | - |
| `routes/devices/[id]/overview/facts/OverviewFacts.svelte` | 2 (2) | 8 | 0 | - |
| `routes/devices/[id]/overview/facts/Reading.svelte` | 1 (1) | 1 | 0 | - |
| `routes/devices/[id]/overview/facts/StatusDot.svelte` | 1 (1) | 10 | 0 | - |

### `routes/devices/[id]/sessions`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/sessions/SessionsPanel.svelte` | 2 (1) | 20 | 0 | - |

### `routes/devices/[id]/settings`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/settings/SettingsPanel.svelte` | 2 (1) | 11 | 1 | - |

### `routes/devices/[id]/tabs`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/devices/[id]/tabs/TabPanel.svelte` | 2 (1) | 7 | 0 | - |
| `routes/devices/[id]/tabs/Tabs.svelte` | 2 (1) | 27 | 5 | - |

### `routes/settings`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/settings/+page.svelte` | route | 6 | 2 | - |

### `routes/sign-in`

| Component | Users | Var | Calc | Bypass |
|---|---|---|---|---|
| `routes/sign-in/+page.svelte` | route | 36 | 4 | raw length (size (no token family)) x1 |

## Summary

### Top 20 worst offenders

Ranked by literals plus raw breakpoints, then by `calc` uses (the calc count breaks ties and shows where the token scale is stretched).

| # | Component | Literals | Breakpoints | Calc | What |
|---|---|---|---|---|---|
| 1 | `lib/devices/screen-sharing/ScreenSharingButton.svelte` | 4 | 1 | 3 | raw border style (border/outline) x1; raw length (size (no token family)) x3; raw breakpoint x1 |
| 2 | `lib/web-interfaces/tab/Toolbar.svelte` | 4 | 1 | 1 | raw length (size (no token family)) x4; raw breakpoint x1 |
| 3 | `lib/devices/actions/WarningBadge.svelte` | 2 | 1 | 0 | raw length (size (no token family)) x2; raw breakpoint x1 |
| 4 | `routes/devices/[id]/header/Header.svelte` | 0 | 2 | 6 | raw breakpoint x2 |
| 5 | `lib/vpn/enrol/EnrolModal.svelte` | 1 | 1 | 2 | raw length (other) x1; raw breakpoint x1 |
| 6 | `lib/vpn/keys/RenewKey.svelte` | 1 | 1 | 2 | raw length (other) x1; raw breakpoint x1 |
| 7 | `lib/vpn/sites/ApproveRoute.svelte` | 1 | 1 | 2 | raw length (other) x1; raw breakpoint x1 |
| 8 | `routes/+layout.svelte` | 2 | 0 | 1 | raw length (size (no token family)) x2 |
| 9 | `lib/sessions/live/LiveSessions.svelte` | 1 | 0 | 5 | raw length (size (no token family)) x1 |
| 10 | `lib/devices/toolbar/Menu.svelte` | 1 | 0 | 4 | raw length (size (no token family)) x1 |
| 11 | `lib/notifications/channels/ChannelRow.svelte` | 0 | 1 | 4 | raw breakpoint x1 |
| 12 | `lib/settings/SettingRow.svelte` | 0 | 1 | 4 | raw breakpoint x1 |
| 13 | `routes/sign-in/+page.svelte` | 1 | 0 | 4 | raw length (size (no token family)) x1 |
| 14 | `lib/devices/actions/WebMenu.svelte` | 1 | 0 | 3 | raw length (size (no token family)) x1 |
| 15 | `lib/nav/Nav.svelte` | 1 | 0 | 3 | raw length (size (no token family)) x1 |
| 16 | `lib/notifications/settings/SettingsForm.svelte` | 0 | 1 | 3 | raw breakpoint x1 |
| 17 | `lib/sessions/view/pane/FindBar.svelte` | 1 | 0 | 3 | raw length (size (no token family)) x1 |
| 18 | `lib/api-keys/KeySecret.svelte` | 0 | 1 | 2 | raw breakpoint x1 |
| 19 | `lib/sessions/wake/WakeButton.svelte` | 1 | 0 | 1 | raw length (size (no token family)) x1 |
| 20 | `lib/updates/UpdateBanner.svelte` | 1 | 0 | 1 | raw length (size (no token family)) x1 |

### Token families most bypassed

| Bypass | Count |
|---|---|
| raw length (size (no token family)) | 19 |
| raw breakpoint (@media/@container) | 16 |
| raw length (other) | 3 |
| raw border style (border/outline) | 2 |
| raw length (type: line-height) | 1 |

Token families by use, for scale (references in `<style>` blocks): `space` 696, `text` 477, `border` 443, `accent` 142, `radius` 131, `surface` 131, `status` 131, `target` 116, `leading` 111, `duration` 63, `ease` 49, `font` 41, `bg` 25, `weight` 19.

Reading: there is no `size` or `breakpoint` token family, so widths (`flex-basis`, `1px` hit areas, `rem` container breakpoints) are written as literals, and the `calc` expressions combine `--space-*` steps because the scale has no step in between. These two gaps, not colour, are where the components leave the tokens.

### Components with 0 or 1 users

A component with one importer is a single-use unit (CUPID "Composable": no single-use interfaces); with zero, check that tests or a route do not mount it by a path this script cannot see.

| Component | Users | Production users |
|---|---|---|
| `lib/updates/tests/LogRowStub.svelte` | 0 | 0 |
| `lib/updates/tests/RowStub.svelte` | 0 | 0 |
| `lib/api-keys/KeyRow.svelte` | 1 | 1 |
| `lib/api-keys/NewKeyDialog.svelte` | 1 | 1 |
| `lib/checks/form/channels/ChannelsSection.svelte` | 1 | 1 |
| `lib/checks/form/fields/AdvancedFields.svelte` | 1 | 1 |
| `lib/checks/form/fields/GroupFields.svelte` | 1 | 1 |
| `lib/checks/form/fields/HttpFields.svelte` | 1 | 1 |
| `lib/checks/form/fields/TypeFields.svelte` | 1 | 1 |
| `lib/checks/form/probes/DomainExpiryFields.svelte` | 1 | 1 |
| `lib/checks/form/probes/ProbeFields.svelte` | 1 | 1 |
| `lib/checks/form/probes/SnmpFields.svelte` | 1 | 1 |
| `lib/devices/add/MachineChoice.svelte` | 1 | 1 |
| `lib/devices/card/DeviceGrid.svelte` | 1 | 1 |
| `lib/devices/form/form/GeneralSection.svelte` | 1 | 1 |
| `lib/devices/form/form/SshSection.svelte` | 1 | 1 |
| `lib/devices/form/form/TagsField.svelte` | 1 | 1 |
| `lib/devices/form/form/TestConnection.svelte` | 1 | 1 |
| `lib/devices/form/policy/PolicyNotice.svelte` | 1 | 1 |
| `lib/devices/form/policy/PolicySection.svelte` | 1 | 1 |
| `lib/devices/form/remote/RdpSection.svelte` | 1 | 1 |
| `lib/devices/form/remote/TransferSection.svelte` | 1 | 1 |
| `lib/devices/form/remote/VncSection.svelte` | 1 | 1 |
| `lib/devices/form/remote/test/RemoteTest.svelte` | 1 | 1 |
| `lib/devices/form/web/Suggestions.svelte` | 1 | 1 |
| `lib/devices/form/web/WebInterfaceRow.svelte` | 1 | 1 |
| `lib/events/panel/LogFilters.svelte` | 1 | 1 |
| `lib/forms/modal/tests/FooterActions.svelte` | 1 | 0 |
| `lib/forms/modal/tests/FooterHost.svelte` | 1 | 0 |
| `lib/hub-address/HubRows.svelte` | 1 | 1 |
| `lib/metrics/agent/ThisComputerButton.svelte` | 1 | 1 |
| `lib/nav/Nav.svelte` | 1 | 1 |
| `lib/nav/collapse/CollapseToggle.svelte` | 1 | 1 |
| `lib/nav/collapse/ResizeEdge.svelte` | 1 | 1 |
| `lib/notifications/channels/ChannelList.svelte` | 1 | 1 |
| `lib/notifications/channels/ChannelRow.svelte` | 1 | 1 |
| `lib/notifications/editor/ChannelEditor.svelte` | 1 | 1 |
| `lib/notifications/editor/ChannelForm.svelte` | 1 | 1 |
| `lib/notifications/editor/kinds/AppriseFields.svelte` | 1 | 1 |
| `lib/notifications/editor/kinds/EmailFields.svelte` | 1 | 1 |
| `lib/notifications/editor/kinds/NtfyFields.svelte` | 1 | 1 |
| `lib/notifications/editor/kinds/WebhookFields.svelte` | 1 | 1 |
| `lib/notifications/settings/SettingsForm.svelte` | 1 | 1 |
| `lib/retention/RetentionRow.svelte` | 1 | 1 |
| `lib/sessions/recording/asciicast/AsciicastPlayer.svelte` | 1 | 0 |
| `lib/sessions/strip/SessionPanes.svelte` | 1 | 1 |
| `lib/sessions/strip/SessionTab.svelte` | 1 | 1 |
| `lib/sessions/strip/SessionTabs.svelte` | 1 | 1 |
| `lib/sessions/view/filing/hubbed.fixture.svelte` | 1 | 0 |
| `lib/sessions/view/hostkey/HostKeyModal.svelte` | 1 | 1 |
| `lib/sessions/view/pane/FindBar.svelte` | 1 | 1 |
| `lib/sessions/view/pane/LinkConfirm.svelte` | 1 | 1 |
| `lib/sessions/view/recording/RecordingBadge.svelte` | 1 | 0 |
| `lib/sessions/view/remote/RemoteToolbar.svelte` | 1 | 1 |
| `lib/settings/SettingsSection.svelte` | 1 | 1 |
| `lib/updates/DesktopRows.svelte` | 1 | 1 |
| `lib/vpn/keys/RenewKey.svelte` | 1 | 1 |
| `lib/web-interfaces/tab/BrowserPage.svelte` | 1 | 1 |
| `lib/web-interfaces/tab/DownPage.svelte` | 1 | 1 |
| `lib/web-interfaces/tab/Toolbar.svelte` | 1 | 1 |
| `lib/web-interfaces/tab/TrustPage.svelte` | 1 | 1 |
| `routes/devices/[id]/checks/CheckHistory.svelte` | 1 | 1 |
| `routes/devices/[id]/checks/CheckRow.svelte` | 1 | 1 |
| `routes/devices/[id]/checks/ChecksPanel.svelte` | 1 | 1 |
| `routes/devices/[id]/detail/DetailBody.svelte` | 1 | 1 |
| `routes/devices/[id]/header/DetailChips.svelte` | 1 | 1 |
| `routes/devices/[id]/header/HeaderActions.svelte` | 1 | 1 |
| `routes/devices/[id]/overview/MetricsSection.svelte` | 1 | 1 |
| `routes/devices/[id]/overview/facts/HostKeyCard.svelte` | 1 | 1 |
| `routes/devices/[id]/overview/facts/Reading.svelte` | 1 | 1 |
| `routes/devices/[id]/overview/facts/StatusDot.svelte` | 1 | 1 |

2 components have no importer; 69 have exactly one (5 of them only in tests).

