// @vitest-environment jsdom
import { mount, unmount } from 'svelte';
import { expect, it } from 'vitest';
import { Pages } from '../pages.svelte';
import { session } from '../page.fixture.svelte';
import SessionsPanel from './SessionsPanel.svelte';
import type { DeviceSession } from './sessions';

async function show(sessions: readonly DeviceSession[]): Promise<{ target: HTMLElement; stop: () => void }> {
  const pages = new Pages<DeviceSession>(() =>
    Promise.resolve({ ok: true, value: { pageInfo: { hasNextPage: false }, edges: sessions.map((node) => ({ node })) } }),
  );
  await pages.load();
  const target = document.body.appendChild(document.createElement('div'));
  const view = mount(SessionsPanel, { target, props: { pages, now: () => new Date('2026-11-04T12:00:00Z') } });
  return { target, stop: () => void unmount(view) };
}

it('RA-030, 6.7: the Sessions table has a Protocol column after Duration, with VNC named "VNC"', async () => {
  const { target, stop } = await show([session('v', '2026-11-04T10:00:00Z', { protocol: 'VNC' })]);
  const heads = [...target.querySelectorAll('th')].map((th) => th.textContent);
  expect(heads).toEqual(['Start', 'End', 'Duration', 'Protocol', 'Operator', 'End reason']);
  expect([...target.querySelectorAll('tbody td')][3]?.textContent).toBe('VNC');
  stop();
});

it('6.7: the empty row spans every column', async () => {
  const { target, stop } = await show([]);
  expect(target.querySelector('td.none')?.getAttribute('colspan')).toBe('6');
  stop();
});
