import { describe, expect, it } from 'vitest';
import type { FileEntry } from './link/codec';
import { fraction, joinPath, parentPath, saveName, sizeText, sortedEntries, timeText } from './paths';

describe('RA-032: joinPath', () => {
  it('joins a plain name to a directory with one slash', () => {
    expect(joinPath('/home/ada', 'notes.txt')).toBe('/home/ada/notes.txt');
    expect(joinPath('/', 'etc')).toBe('/etc');
    expect(joinPath('/home/ada/', 'a b')).toBe('/home/ada/a b');
  });

  it.each(['', '.', '..', 'a/b', '../x', '/etc/passwd', 'a\0b'])('refuses the hostile name %j', (name) => {
    expect(joinPath('/home/ada', name)).toBeUndefined();
  });
});

describe('RA-032: parentPath', () => {
  it.each([
    ['/home/ada', '/home'],
    ['/home/ada/', '/home'],
    ['/home', '/'],
    ['/', '/'],
    ['docs/old', 'docs'],
    ['docs', '.'],
  ])('the parent of %s is %s', (path, parent) => {
    expect(parentPath(path)).toBe(parent);
  });
});

describe('RA-032: saveName', () => {
  it.each([
    ['report.pdf', 'report.pdf'],
    ['../../etc/passwd', 'passwd'],
    ['a\\b\\c.txt', 'c.txt'],
    ['bad\u0000na\u001bme\n.txt', 'badname.txt'],
    ['  spaced.txt  ', 'spaced.txt'],
    ['', 'download'],
    ['..', 'download'],
    ['/', 'download'],
    ['dir/', 'download'],
    ['\u0007\u0008', 'download'],
  ])('%j is saved as %j', (name, saved) => {
    expect(saveName(name)).toBe(saved);
  });
});

describe('RA-032: listing text', () => {
  const entry = (name: string, kind: FileEntry['kind']): FileEntry => ({ name, kind, size: null, modifiedAt: null });

  it('puts directories first and sorts each group by name', () => {
    const names = sortedEntries([entry('b.txt', 'file'), entry('zed', 'dir'), entry('A.txt', 'file'), entry('alpha', 'dir'), entry('ln', 'link')]).map(
      (each) => each.name,
    );
    expect(names).toEqual(['alpha', 'zed', 'A.txt', 'b.txt', 'ln']);
  });

  it('shows sizes in binary units and a dash for an unknown size or time', () => {
    expect(sizeText(null)).toBe('—');
    expect(sizeText(0)).toBe('0 B');
    expect(sizeText(1023)).toBe('1023 B');
    expect(sizeText(1536)).toBe('1.5 KiB');
    expect(sizeText(20 * 1024 * 1024)).toBe('20 MiB');
    expect(timeText(null)).toBe('—');
    expect(timeText('not a time')).toBe('—');
    expect(timeText('2026-11-05T10:00:00Z', 'en')).toMatch(/2026/);
  });

  it('MOT-009: progress is bytes over size, linear, and never above 1', () => {
    expect(fraction(0, 200)).toBe(0);
    expect(fraction(50, 200)).toBe(0.25);
    expect(fraction(200, 200)).toBe(1);
    expect(fraction(300, 200)).toBe(1);
    expect(fraction(5, undefined)).toBe(0);
    expect(fraction(0, 0)).toBe(0);
  });
});
