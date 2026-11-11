// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import { node, NOW } from '../vpn.fixture';
import DeviceVpn from './DeviceVpn.svelte';
import { device } from '../../../routes/devices/[id]/page.fixture.svelte';
import OverviewPanel from '../../../routes/devices/[id]/overview/OverviewPanel.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const words = (element: Element | null | undefined) => (element?.textContent ?? '').replace(/\p{Co}/gu, '').replace(/\s+/g, ' ').trim();

function overview(vpn: ReturnType<typeof node> | null): void {
  app = mount(OverviewPanel, {
    target: document.body,
    props: {
      device: device({ metricsSource: 'NONE' }),
      checks: [],
      metrics: { view: null, span: null, shownRange: null, chartShape: null, readings: null, error: null, liveError: null },
      range: 'HOUR',
      vpn,
      now: () => NOW,
      onRange: () => undefined,
      onRetry: () => undefined,
      onSettings: () => undefined,
    },
  });
  flushSync();
}

const vpnCard = () => [...document.querySelectorAll('section')].find((one) => words(one.querySelector('h2')) === 'VPN');

it('VPN-005: the device Overview shows address, path and key expiry', () => {
  overview(node({ keyState: 'EXPIRING', keyExpiry: '2026-11-19T12:00:00Z', path: 'RELAYED', relayRegion: 'sfo' }));
  const rows = [...(vpnCard()?.querySelectorAll('dt') ?? [])].map((term) => [words(term), words(term.nextElementSibling)]);
  expect(rows).toEqual([
    ['Address', '100.64.0.4'],
    ['Path', 'Relayed (sfo)'],
    ['Key', 'Expires in 9 days'],
    ['OS', 'macOS'],
  ]);
  expect(vpnCard()?.getAttribute('aria-labelledby')).toBe(vpnCard()?.querySelector('h2')?.id);
});

it('VPN-005: a device with no VPN node shows no VPN card', () => {
  overview(null);
  expect(vpnCard()).toBeUndefined();
  expect(document.querySelector('section[aria-labelledby]')).not.toBeNull();
});

it('VPN-005: the card stands alone with the node it is given', () => {
  app = mount(DeviceVpn, { target: document.body, props: { node: node({ keyState: 'NEVER', keyExpiry: null }), now: NOW } });
  flushSync();
  expect(document.body.textContent).toContain('Does not expire');
});
