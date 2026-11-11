// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { node, NOW, STRANGER } from '../vpn.fixture';
import NodeKeys from './NodeKeys.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(nodes: ReturnType<typeof node>[]): HTMLElement[] {
  const hub = node({ name: 'hub', address: '100.64.0.1', role: 'HUB', keyState: 'NEVER', keyExpiry: null, inAnima: false, device: null });
  app = mount(NodeKeys, { target: document.body, props: { hub, nodes, now: NOW } });
  flushSync();
  return [...document.querySelectorAll<HTMLElement>('ul[aria-label="VPN nodes"] > li')];
}

it('VPN-010: each node row has its name, key words and the tone as a word beside a dot', () => {
  const [, row] = show([node({ keyState: 'EXPIRING', keyExpiry: '2026-11-19T12:00:00Z' })]);
  expect(row?.getAttribute('aria-label')).toBe('mac');
  expect(row?.querySelector('.key')?.textContent).toBe('Expires in 9 days');
  expect(row?.querySelector('.key')?.classList.contains('warning')).toBe(true);
  expect(row?.querySelector('.dot')?.getAttribute('aria-hidden')).toBe('true');
});

it('VPN-011: the hub is first, then a node not in Anima, marked in text, then the others', () => {
  const rows = show([node({ name: 'aaa' }), STRANGER]);
  expect(rows.map((row) => row.getAttribute('aria-label'))).toEqual(['hub', 'stranger', 'aaa']);
  expect(rows[1]?.textContent).toContain('Not in Anima');
  expect(rows[0]?.textContent).not.toContain('Not in Anima');
  expect(rows[2]?.textContent).not.toContain('Not in Anima');
});
