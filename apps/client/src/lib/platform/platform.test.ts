import { readdirSync, readFileSync } from 'node:fs';
import { posix } from 'node:path';
import { fileURLToPath } from 'node:url';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { desktopCapabilities } from '$lib/desktop/capabilities';
import { DESKTOP_MAC } from './answers.fixture';
import { browserCapabilities } from './browser';
import { isMac } from './mac';
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

  it('R-059, R-079, NAV-007: on Linux the desktop app has no Sharing settings to open and keeps its own title bar', () => {
    expect(desktopCapabilities(LINUX)).toEqual({ ...DESKTOP_MAC, systemSettings: false, overlayTitleBar: false });
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

describe('isMac (BLD-068, R-059)', () => {
  it('BLD-068: a Mac user agent is a Mac; Linux, Windows, an iPhone, an iPad and an empty one are not', () => {
    expect(isMac(MAC)).toBe(true);
    expect(isMac('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko)')).toBe(true);
    expect(isMac(LINUX)).toBe(false);
    expect(isMac('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe(false);
    expect(isMac('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15')).toBe(false);
    expect(isMac('Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15')).toBe(false);
    expect(isMac('')).toBe(false);
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
    const importers = sources.filter(({ path, text }) => importsUpdaterValues(path, text)).map(({ path }) => path);
    expect(importers).toEqual([]);
    const lazy = sources.filter(({ text }) => /import\(\s*['"`][^'"`]*desktop\/updater['"`]\s*\)/.test(text)).map(({ path }) => path);
    expect(lazy).toEqual(['lib/updates/desktop.svelte.ts']);
  });

  it('UPD-001: the scan resolves each specifier against the file, catches value imports and re-exports, and lets type imports through', () => {
    const scan = (text: string, path = 'lib/updates/x.ts'): boolean => importsUpdaterValues(path, text);
    expect(scan("import { desktopUpdater } from '$lib/desktop/updater';")).toBe(true);
    expect(scan("import { desktopUpdater } from '../desktop/updater';")).toBe(true);
    expect(scan("import { desktopUpdater } from '../../../desktop/updater.ts';", 'lib/a/b/c/x.ts')).toBe(true);
    expect(scan("import { desktopUpdater } from '../../desktop/updater';", 'lib/a/b/c/x.ts')).toBe(false);
    expect(scan("export { desktopUpdater } from '$lib/desktop/updater';")).toBe(true);
    expect(scan("export * from '../desktop/updater';")).toBe(true);
    expect(scan("import '$lib/desktop/updater';")).toBe(true);
    expect(scan("import type { UpdaterPort } from '../desktop/updater';")).toBe(false);
    expect(scan("export type { UpdaterPort } from '$lib/desktop/updater';")).toBe(false);
    // From inside lib/desktop the same folder is reached by './updater'.
    expect(scan("export { desktopUpdater } from './updater';", 'lib/desktop/x.ts')).toBe(true);
    expect(scan("export { desktopUpdater } from './updater.js';", 'lib/desktop/x.ts')).toBe(true);
    expect(scan("export type { UpdaterPort } from './updater';", 'lib/desktop/x.ts')).toBe(false);
    // './updater' elsewhere is another module; the updater's own file is skipped.
    expect(scan("import { x } from './updater';", 'lib/other/x.ts')).toBe(false);
    expect(scan("import { x } from './updater';", 'lib/desktop/updater.ts')).toBe(false);
  });

  it("UPD-001: a comment with an apostrophe between 'import' and 'from' does not hide the specifier", () => {
    const text = "import {\n  // the port's value\n  desktopUpdater,\n} from './updater';\n/* it's */";
    expect(importsUpdaterValues('lib/desktop/x.ts', text)).toBe(true);
    expect(importsUpdaterValues('lib/desktop/x.ts', "// import { a } from './updater';")).toBe(false);
  });
});

/** The module path a specifier names, as `lib/...` without extension; null for a package. */
function resolveSpecifier(file: string, specifier: string): string | null {
  const target = specifier.startsWith('$lib/')
    ? `lib/${specifier.slice('$lib/'.length)}`
    : specifier.startsWith('.')
      ? posix.join(posix.dirname(file), specifier)
      : null;
  return target?.replace(/\.(?:ts|js)$/, '') ?? null;
}

/** `text` without its comments; strings are kept whole, so an apostrophe in a comment cannot open one. */
function withoutComments(text: string): string {
  return text.replace(/(['"`])(?:\\.|(?!\1)[^\\])*\1|\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, (match) => (/^(?:\/\/|\/\*)/.test(match) ? ' ' : match));
}

/** Whether the file at `path` imports or re-exports values of `lib/desktop/updater` (its own file and type-only forms excepted). */
function importsUpdaterValues(path: string, text: string): boolean {
  if (/^lib\/desktop\/updater\.(?:ts|js)$/.test(path)) return false;
  const statement = /\b(?:import|export)\s+(?!type\b)(?:[^;'"]*?\bfrom\s+)?['"]([^'"]+)['"]/g;
  return [...withoutComments(text).matchAll(statement)].some(([, specifier]) => resolveSpecifier(path, specifier ?? '') === 'lib/desktop/updater');
}
