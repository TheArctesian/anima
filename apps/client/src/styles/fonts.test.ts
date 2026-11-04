import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const stylesDir = fileURLToPath(new URL('./', import.meta.url));
const fontsDir = `${stylesDir}fonts/`;
const pinnedCommit = 'f034eeafc0ccf79afa8dd952feb86b18e5d0469e';

interface LockEntry {
  file: string;
  url: string;
  sha256: string;
  bytes: number;
  licence?: string;
}

interface FontLock {
  commit: string;
  files: LockEntry[];
}

interface FontFace {
  family: string;
  weight: number;
  style: string;
  display: string;
  file: string;
  format: string;
}

const lock = JSON.parse(readFileSync(`${fontsDir}fonts.lock.json`, 'utf8')) as FontLock;
const fontsCss = readFileSync(`${stylesDir}fonts.css`, 'utf8');
const globalCss = readFileSync(`${stylesDir}global.css`, 'utf8');
const sha256 = (path: string) => createHash('sha256').update(readFileSync(path)).digest('hex');

const stripComments = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, '');
const unquote = (value: string) => value.replace(/^["']|["']$/g, '');

function parseFontFaces(css: string): FontFace[] {
  return [...stripComments(css).matchAll(/@font-face\s*\{([^}]*)\}/g)].map((block) => {
    const declarations = new Map<string, string>();
    for (const declaration of (block[1] ?? '').split(';')) {
      const colon = declaration.indexOf(':');
      if (colon >= 0) {
        declarations.set(declaration.slice(0, colon).trim().toLowerCase(), declaration.slice(colon + 1).trim());
      }
    }
    const source = /^url\("\.\/fonts\/([^"]+)"\)\s+format\("([^"]+)"\)$/.exec(declarations.get('src') ?? '');
    return {
      family: unquote(declarations.get('font-family') ?? ''),
      weight: Number(declarations.get('font-weight')),
      style: declarations.get('font-style') ?? '',
      display: declarations.get('font-display') ?? '',
      file: source?.[1] ?? '',
      format: source?.[2] ?? '',
    };
  });
}

const byFace = (a: FontFace, b: FontFace) =>
  `${a.family}|${a.weight}|${a.style}`.localeCompare(`${b.family}|${b.weight}|${b.style}`);

const propo = 'OpenDyslexic Nerd Font Propo';
const mono = 'OpenDyslexicM Nerd Font Mono';
const symbols = 'Symbols Nerd Font';
const expectedFaces: FontFace[] = [
  { family: propo, weight: 400, style: 'normal', display: 'swap', file: 'OpenDyslexicNerdFontPropo-Regular.otf', format: 'opentype' },
  { family: propo, weight: 700, style: 'normal', display: 'swap', file: 'OpenDyslexicNerdFontPropo-Bold.otf', format: 'opentype' },
  { family: propo, weight: 400, style: 'italic', display: 'swap', file: 'OpenDyslexicNerdFontPropo-Italic.otf', format: 'opentype' },
  { family: propo, weight: 700, style: 'italic', display: 'swap', file: 'OpenDyslexicNerdFontPropo-BoldItalic.otf', format: 'opentype' },
  { family: mono, weight: 400, style: 'normal', display: 'swap', file: 'OpenDyslexicMNerdFontMono-Regular.otf', format: 'opentype' },
  { family: symbols, weight: 400, style: 'normal', display: 'swap', file: 'SymbolsNerdFont-Regular.ttf', format: 'truetype' },
];

describe('@font-face declarations (UI-010, UI-011, UI-014)', () => {
  it('declares exactly the six faces the design uses, each with its file', () => {
    expect([...parseFontFaces(fontsCss)].sort(byFace)).toEqual([...expectedFaces].sort(byFace));
  });

  it('uses font-display swap on every face', () => {
    for (const face of parseFontFaces(fontsCss)) {
      expect(face.display, `${face.family} ${face.weight} ${face.style}`).toBe('swap');
    }
  });
});

describe('bundled font files (UI-022)', () => {
  it('pins every lock entry to the nerd-fonts commit', () => {
    expect(lock.commit).toBe(pinnedCommit);
    for (const entry of lock.files) {
      expect(entry.url, entry.file).toContain(`/nerd-fonts/${pinnedCommit}/`);
    }
  });

  it('has a file on disk for every lock entry, with matching SHA-256 and size', () => {
    expect(lock.files.length).toBeGreaterThan(0);
    for (const entry of lock.files) {
      const path = `${fontsDir}${entry.file}`;
      expect(existsSync(path), entry.file).toBe(true);
      expect(sha256(path), entry.file).toBe(entry.sha256);
      expect(statSync(path).size, entry.file).toBe(entry.bytes);
    }
  });

  it('keeps exactly the lock-listed files and the lock file in fonts/', () => {
    const onDisk = readdirSync(fontsDir).sort();
    const expected = [...lock.files.map((entry) => entry.file), 'fonts.lock.json'].sort();
    expect(onDisk).toEqual(expected);
  });
});

describe('fonts load from the application package (UI-017)', () => {
  const urls = [...stripComments(fontsCss).matchAll(/url\(\s*"([^"]+)"\s*\)/g)].map((m) => m[1] ?? '');

  it('declares at least one font file in fonts.css', () => {
    expect(urls.length).toBeGreaterThan(0);
  });

  it('uses only relative url() paths', () => {
    for (const url of urls) {
      expect(url.startsWith('./'), url).toBe(true);
      expect(url, url).not.toMatch(/^[a-z]+:/i);
    }
  });

  it('points every url() at a file listed in the lock file', () => {
    const locked = new Set(lock.files.map((entry) => entry.file));
    for (const url of urls) {
      expect(locked.has(url.replace(/^\.\/fonts\//, '')), url).toBe(true);
    }
  });

  it('has no http(s) url() or @import in any style sheet', () => {
    const remote = /(url\(\s*["']?|@import\s+(url\(\s*)?["']?)\s*https?:/i;
    expect(remote.test('@import "https://example.test/a.css";')).toBe(true);
    expect(remote.test('a { background: url(http://example.test/a.png); }')).toBe(true);
    const sheets = readdirSync(stylesDir).filter((name) => name.endsWith('.css'));
    expect(sheets).toContain('fonts.css');
    for (const sheet of sheets) {
      expect(readFileSync(`${stylesDir}${sheet}`, 'utf8'), sheet).not.toMatch(remote);
    }
  });

  it('imports tokens.css and then fonts.css from global.css', () => {
    const imports = [...stripComments(globalCss).matchAll(/@import\s+["']([^"']+)["']\s*;/g)].map((m) => m[1]);
    expect(imports).toEqual(['./tokens.css', './fonts.css']);
  });
});

describe('font licences (UI-018)', () => {
  for (const licence of ['LICENSE-OpenDyslexic.txt', 'LICENSE-SymbolsNerdFont.txt', 'LICENSE-NerdFonts.txt']) {
    it(`ships ${licence} as a non-empty file`, () => {
      const path = `${fontsDir}${licence}`;
      expect(existsSync(path)).toBe(true);
      expect(statSync(path).size).toBeGreaterThan(0);
    });
  }

  it('lists a licence file for every font family in fonts.css', () => {
    const faces = parseFontFaces(fontsCss);
    for (const family of new Set(faces.map((face) => face.family))) {
      const files = faces.filter((face) => face.family === family).map((face) => face.file);
      for (const file of files) {
        const entry = lock.files.find((listed) => listed.file === file);
        expect(entry?.licence, `${family}: ${file}`).toBeDefined();
        const licence = lock.files.find((listed) => listed.file === entry?.licence);
        expect(licence, `${family}: ${entry?.licence}`).toBeDefined();
        expect(statSync(`${fontsDir}${licence?.file}`).size, licence?.file).toBeGreaterThan(0);
      }
    }
  });
});
