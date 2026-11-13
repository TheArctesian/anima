// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({ platform: undefined as Capabilities | undefined, goto: vi.fn(() => Promise.resolve()) }));

vi.mock('$lib/platform/platform', () => ({ capabilities: () => mocks.platform }));
vi.mock('$app/navigation', () => ({ goto: mocks.goto }));
// The view loads its own data and has its own tests in `$lib/builds/view/tests/`.
vi.mock('$lib/builds/view/BuildsView.svelte', () => ({ default: () => undefined }));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  mocks.goto.mockClear();
});

function open(platform: Capabilities): void {
  mocks.platform = platform;
  app = mount(Page, { target: document.body });
  flushSync();
}

it('BLD-080: in a browser the Builds page sends the operator to Devices', () => {
  open(BROWSER);
  expect(mocks.goto).toHaveBeenCalledWith('/devices', { replaceState: true });
});

it('BLD-080: in the desktop app the Builds page stays', () => {
  open(DESKTOP_MAC);
  expect(mocks.goto).not.toHaveBeenCalled();
});
