// Test support: a screen that draws nothing, for tests of the tab strip, which are not about xterm.js. Use it as
// `vi.mock('$lib/sessions/terminal/screen', async () => ({ openScreen: (await import('./silent.fixture')).silentScreen }))`.
import type { ScreenOpener } from './screen';

export const silentScreen: ScreenOpener = () =>
  Promise.resolve({
    size: () => ({ cols: 80, rows: 24 }),
    write: () => undefined,
    reset: () => undefined,
    fit: () => undefined,
    focus: () => undefined,
    blur: () => undefined,
    paste: () => undefined,
    find: () => false,
    clearFind: () => undefined,
    dispose: () => undefined,
  });
