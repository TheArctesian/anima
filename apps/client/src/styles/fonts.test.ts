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
}

interface FontLock {
  commit: string;
  files: LockEntry[];
}

const lock = JSON.parse(readFileSync(`${fontsDir}fonts.lock.json`, 'utf8')) as FontLock;
const fontsCss = readFileSync(`${stylesDir}fonts.css`, 'utf8');
const sha256 = (path: string) => createHash('sha256').update(readFileSync(path)).digest('hex');

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
});

describe('fonts load from the application package (UI-017)', () => {
  const urls = [...fontsCss.matchAll(/url\(\s*"([^"]+)"\s*\)/g)].map((m) => m[1] ?? '');

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

  it('has no http reference in any style sheet', () => {
    const sheets = readdirSync(stylesDir).filter((name) => name.endsWith('.css'));
    expect(sheets).toContain('fonts.css');
    for (const sheet of sheets) {
      expect(readFileSync(`${stylesDir}${sheet}`, 'utf8'), sheet).not.toMatch(/http/i);
    }
  });
});

describe('font licences are in the package (UI-018)', () => {
  for (const licence of ['OFL.txt', 'LICENSE-NerdFonts.txt']) {
    it(`ships ${licence} as a non-empty file`, () => {
      const path = `${fontsDir}${licence}`;
      expect(existsSync(path)).toBe(true);
      expect(statSync(path).size).toBeGreaterThan(0);
    });
  }
});
