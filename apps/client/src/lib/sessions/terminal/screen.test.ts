// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { openScreen } from './screen';

const webgl = vi.hoisted(() => ({ disposed: 0, loseContext: undefined as (() => void) | undefined }));
vi.mock('@xterm/addon-webgl', () => ({
  WebglAddon: class {
    activate() {}
    dispose() {
      webgl.disposed += 1;
    }
    onContextLoss(listener: () => void) {
      webgl.loseContext = listener;
    }
  },
}));

const hooks = { oninput: vi.fn(), onresize: vi.fn(), oncopy: vi.fn(), onfind: vi.fn(), ontitle: vi.fn(), onlink: vi.fn() };
let saved: typeof window.matchMedia;

beforeEach(() => {
  webgl.disposed = 0;
  saved = window.matchMedia;
  window.matchMedia = (() => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined, addListener: () => undefined, removeListener: () => undefined })) as unknown as typeof window.matchMedia;
});

afterEach(() => {
  window.matchMedia = saved;
  document.body.innerHTML = '';
});

async function opened() {
  const host = document.createElement('div');
  document.body.append(host);
  const screen = await openScreen(host, hooks);
  return { host, screen };
}

it('D-4: the DOM renderer takes over at once when the canvas reports a lost WebGL context, not after the addon\'s 3 s wait', async () => {
  const { host } = await opened();
  const canvas = document.createElement('canvas');
  host.querySelector('.xterm-screen')?.append(canvas);
  canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
  expect(webgl.disposed).toBe(1);
});

it('D-4: the addon\'s own context loss report is the backstop, and the addon is disposed only once', async () => {
  const { host } = await opened();
  webgl.loseContext?.();
  host.querySelector('.xterm-screen')?.dispatchEvent(new Event('webglcontextlost'));
  webgl.loseContext?.();
  expect(webgl.disposed).toBe(1);
});
