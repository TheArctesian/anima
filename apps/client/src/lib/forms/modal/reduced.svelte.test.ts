// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { installAnimations, played } from '../animations.fixture';
import ConfirmModal from './ConfirmModal.svelte';

vi.hoisted(() => {
  // The window's preference is read once, so it must be set before the motion module loads (MOT-041).
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: query.includes('reduce'),
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  }));
});

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('Modal, MOT-040: under reduced motion the surface fades and does not scale, and Escape still closes', () => {
  installAnimations();
  played.length = 0;
  const onCancel = vi.fn();
  const state = $state({ open: false });
  app = mount(ConfirmModal, {
    target: document.body,
    props: {
      get open() {
        return state.open;
      },
      title: 'Delete nas?',
      body: 'Its credentials go too.',
      confirmLabel: 'Delete nas',
      onConfirm: () => Promise.resolve({ ok: true as const, value: undefined }),
      onCancel,
    },
  });
  flushSync();
  state.open = true;
  flushSync();

  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

  expect(onCancel).toHaveBeenCalledOnce();
  const keyframes = played.flatMap((call) => call.keyframes.map((frame) => Object.keys(frame)));
  expect(keyframes.length).toBeGreaterThan(0);
  expect(keyframes.flat()).toContain('opacity');
  expect(keyframes.flat()).not.toContain('scale');
});
