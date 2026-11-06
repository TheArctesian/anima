// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import { device } from '../page.fixture.svelte';
import OverviewPanel from './OverviewPanel.svelte';
import { checkCounts, hostKeyText, uptimeReadings } from './overview';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('D-10: uptime for 24 hours, 7 days and 30 days; no value is "—"', () => {
  expect(uptimeReadings(device({ day: 0.9995, week: 0.5, month: null }))).toEqual([
    { label: '24 hours', text: '99.95 %' },
    { label: '7 days', text: '50.00 %' },
    { label: '30 days', text: '—' },
  ]);
});

it('D-10: checks are counted by status, worst first, without empty statuses', () => {
  expect(checkCounts([{ status: 'UP' }, { status: 'DOWN' }, { status: 'UP' }])).toEqual([
    { status: 'DOWN', label: 'Down', count: 1 },
    { status: 'UP', label: 'Up', count: 2 },
  ]);
  expect(checkCounts([])).toEqual([]);
});

it.each([
  [{ enabled: true, hostKey: device().ssh.hostKey }, 'ssh-ed25519 SHA256:abc'],
  [{ enabled: true, hostKey: null }, 'No host key accepted yet'],
  [{ enabled: false, hostKey: null }, 'SSH is off'],
])('D-10: the host key line for %j is %j', (ssh, text) => {
  expect(hostKeyText(ssh)).toBe(text);
});

it('D-10: the panel shows status, maintenance, uptime, check counts and the key, and no chart slot', () => {
  app = mount(OverviewPanel, { target: document.body, props: { device: device({ maintenance: true }), checks: [{ status: 'UP' }] } });
  flushSync();
  const text = document.body.textContent;
  for (const part of ['Status', 'Up', 'Maintenance', 'On', 'Uptime, 30 days', '—', '1 Up', 'SHA256:abc']) expect(text).toContain(part);
  expect(document.querySelector('svg, canvas')).toBeNull();
});
