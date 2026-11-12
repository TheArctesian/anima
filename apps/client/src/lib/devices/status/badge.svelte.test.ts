// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import { endTransitionsAtOnce } from '../transitions.fixture';
import StatusBadge from './StatusBadge.svelte';
import type { CombinedStatus } from './precedence';

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(status: CombinedStatus, extra: Record<string, unknown> = {}): HTMLElement | null {
  app = mount(StatusBadge, { target: document.body, props: { status, deviceId: 'd1', ...extra } });
  flushSync();
  return document.querySelector('.pill');
}

it.each<[CombinedStatus, string]>([
  ['UP', 'Up'],
  ['DOWN', 'Down'],
  ['WARNING', 'Warning'],
  ['DEGRADED', 'Degraded'],
  ['MAINTENANCE', 'Maintenance'],
  ['UNKNOWN', 'Unknown'],
])('UI-003: the %s badge always shows the word %s', (status, word) => {
  expect(show(status)?.textContent.trim()).toBe(word);
});

it.each<[CombinedStatus, boolean]>([
  ['UP', true],
  ['UNKNOWN', true],
  ['DOWN', false],
  ['WARNING', false],
  ['DEGRADED', false],
  ['MAINTENANCE', false],
])('DEV-006: %s is quiet (no fill): %s', (status, quiet) => {
  expect(show(status)?.classList.contains('quiet')).toBe(quiet);
});

it('DEV-006: the reasons slot sits beside the badge, so the badge text stays the word alone', () => {
  const reasons = createRawSnippet(() => ({ render: () => '<button>Why</button>' }));
  expect(show('DOWN', { reasons })?.textContent.trim()).toBe('Down');
  expect(document.querySelector('.reasons button')?.textContent).toBe('Why');
});
