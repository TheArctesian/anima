import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { desktopCapabilities } from '$lib/desktop/capabilities';
import { DESKTOP_MAC } from './answers.fixture';
import { browserCapabilities } from './browser';
import { capabilities } from './platform';

const shell = vi.hoisted(() => ({ isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: shell.isTauri, invoke: vi.fn() }));

const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15';
const LINUX = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15';

beforeEach(() => {
  shell.isTauri.mockReturnValue(true);
});

describe('desktopCapabilities (WP-42 D-4)', () => {
  it('CLI-001, CLI-003, UPD-001, LOG-008, R-059: the desktop app on a Mac has every desktop feature but pop-out', () => {
    expect(desktopCapabilities(MAC)).toEqual(DESKTOP_MAC);
  });

  it('R-059, R-079: on Linux the desktop app has no Sharing settings to open; the rest is the same', () => {
    expect(desktopCapabilities(LINUX)).toEqual({ ...DESKTOP_MAC, systemSettings: false });
  });

  it('RA-002: the shell has no pop-out window command, so pop-out is no on every desktop', () => {
    expect(desktopCapabilities(MAC)?.popOut).toBe(false);
    expect(desktopCapabilities(LINUX)?.popOut).toBe(false);
  });

  it('CLI-001: outside the desktop shell there are no desktop answers', () => {
    shell.isTauri.mockReturnValue(false);
    expect(desktopCapabilities(MAC)).toBeNull();
  });
});

describe('capabilities (WP-42 D-4)', () => {
  it('CLI-001: inside the desktop shell the client answers as the shell does', () => {
    vi.stubGlobal('navigator', { userAgent: MAC });
    try {
      expect(capabilities()).toEqual(DESKTOP_MAC);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('CLI-001, UPD-001: outside the desktop shell every answer is no', () => {
    shell.isTauri.mockReturnValue(false);
    expect(capabilities()).toEqual(browserCapabilities);
    expect(Object.values(capabilities()).every((answer) => !answer)).toBe(true);
    expect(Object.keys(capabilities()).sort()).toEqual(['keyStore', 'popOut', 'processLog', 'systemSettings', 'thisMachine', 'updates', 'webViewTabs']);
  });
});

describe('the seam holds (WP-42 D-4)', () => {
  const src = fileURLToPath(new URL('../../', import.meta.url));
  const sources = readdirSync(src, { recursive: true, encoding: 'utf8' })
    .filter((path) => /\.(?:ts|svelte)$/.test(path) && !/\.test\.ts$|\.fixture/.test(path))
    .map((path) => ({ path: path.replaceAll('\\', '/'), text: readFileSync(`${src}${path}`, 'utf8') }));

  it('CLI-001: only lib/desktop/ asks whether this is the desktop shell', () => {
    const askers = sources.filter(({ path, text }) => !path.startsWith('lib/desktop/') && /\bisTauri\b/.test(text));
    expect(sources.length).toBeGreaterThan(100);
    expect(askers.map(({ path }) => path)).toEqual([]);
  });

  it('UPD-001: the updater module is imported for its values only by the lazy port in lib/updates', () => {
    const value = /import\s+(?!type\b)[^;]*?from\s+['"]\$lib\/desktop\/updater['"]/;
    const importers = sources.filter(({ text }) => value.test(text)).map(({ path }) => path);
    expect(importers).toEqual([]);
    const lazy = sources.filter(({ text }) => text.includes("import('$lib/desktop/updater')")).map(({ path }) => path);
    expect(lazy).toEqual(['lib/updates/desktop.svelte.ts']);
  });
});
