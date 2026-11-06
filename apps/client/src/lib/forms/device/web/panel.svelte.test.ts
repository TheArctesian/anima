// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '../../animations.fixture';
import SettingsPanel from '../../../../routes/devices/[id]/settings/SettingsPanel.svelte';
import { device } from '../../../../routes/devices/[id]/page.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(extra: Record<string, unknown>): void {
  const done = () => Promise.resolve({ ok: true as const, value: undefined });
  app = mount(SettingsPanel, {
    target: document.body,
    props: {
      device: device(),
      revision: 0,
      section: null,
      onSave: done,
      onTest: vi.fn(),
      onAcceptHostKey: done,
      onClearCredential: done,
      onCancel: () => undefined,
      onDelete: done,
      ...extra,
    },
  });
  flushSync();
}

it('D-4: the settings panel shows no Web interfaces section without the prop', () => {
  show({});
  expect(document.body.textContent).not.toContain('Web interfaces');
  expect(document.body.textContent).toContain('Delete device');
});

it('D-5: with the prop the section sits before Delete device', () => {
  const done = () => Promise.resolve({ ok: true as const, value: undefined });
  show({
    webInterfaces: {
      interfaces: [],
      suggestions: [],
      calls: { onCreate: done, onUpdate: done, onDelete: done, onAcceptSuggestion: done, onDismissSuggestion: done },
    },
  });
  const headings = [...document.querySelectorAll('h3')].map((heading) => heading.textContent);
  expect(headings.indexOf('Web interfaces')).toBeGreaterThanOrEqual(0);
  expect(headings.indexOf('Web interfaces')).toBeLessThan(headings.indexOf('Delete device'));
});
