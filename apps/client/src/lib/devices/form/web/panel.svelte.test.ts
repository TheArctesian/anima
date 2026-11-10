// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { metricsApi } from '$lib/metrics/api';
import { installAnimations } from '../../../forms/animations.fixture';
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

function show(): void {
  const done = () => Promise.resolve({ ok: true as const, value: undefined });
  app = mount(SettingsPanel, {
    target: document.body,
    props: {
      device: device(),
      revision: 0,
      section: null,
      calls: {
        onSave: done,
        onAcceptHostKey: done,
        onAcceptServerCertificate: done,
        onClearCredential: done,
        onClearRdpCredential: done,
        onClearVncCredential: done,
        webInterfaces: { onCreate: done, onUpdate: done, onDelete: done, onAcceptSuggestion: done, onDismissSuggestion: done },
      },
      agent: { calls: metricsApi(fakeHub().hub), origin: 'https://hub.test', reveal: false },
      now: () => new Date(),
      onTest: vi.fn(),
      onCancel: () => undefined,
      onDelete: done,
    },
  });
  flushSync();
}

it('WEB-001: the settings panel shows the Web interfaces section before Delete device', () => {
  show();
  const headings = [...document.querySelectorAll('h3')].map((heading) => heading.textContent);
  expect(headings.indexOf('Web interfaces')).toBeGreaterThanOrEqual(0);
  expect(headings.indexOf('Web interfaces')).toBeLessThan(headings.indexOf('Delete device'));
});
