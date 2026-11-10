import { describe, expect, it } from 'vitest';
import { BUILT_RELEASE, parseRelease, targetOfPlatform } from './release';

const HEX = 'f'.repeat(64);
const file = (release: unknown) => ({ release });
const good = { version: '1.2.3', targets: { 'linux-x86_64': { sha256: HEX }, 'macos-aarch64': { sha256: HEX } } };

describe('parseRelease (D-10 c)', () => {
  it('MET-019: reads a version and the sha256 of each target', () => {
    expect(parseRelease(file(good))).toEqual({ version: '1.2.3', sha256: { 'linux-x86_64': HEX, 'macos-aarch64': HEX } });
  });

  it('MET-019: the development form and the committed file mean no release', () => {
    expect(parseRelease({ release: null })).toBeNull();
    expect(BUILT_RELEASE).toBeNull();
  });

  it.each([
    ['not an object', 'text'],
    ['an array', []],
    ['no release key', {}],
    ['a version that is not semver', file({ ...good, version: '1.2' })],
    ['a version with a quote', file({ ...good, version: "1.2.3'" })],
    ['no version', file({ targets: good.targets })],
    ['no targets', file({ version: '1.2.3' })],
    ['an empty target list', file({ version: '1.2.3', targets: {} })],
    ['an unknown target', file({ ...good, targets: { 'windows-x86_64': { sha256: HEX } } })],
    ['a short sha256', file({ ...good, targets: { 'linux-x86_64': { sha256: 'f'.repeat(63) } } })],
    ['an upper-case sha256', file({ ...good, targets: { 'linux-x86_64': { sha256: 'F'.repeat(64) } } })],
    ['a sha256 with a quote', file({ ...good, targets: { 'linux-x86_64': { sha256: `${'f'.repeat(63)}'` } } })],
    ['a sha256 that is not text', file({ ...good, targets: { 'linux-x86_64': { sha256: 7 } } })],
    ['one bad target among good ones', file({ ...good, targets: { ...good.targets, 'linux-aarch64': { sha256: 'x' } } })],
  ])('MET-019: %s means no release', (_name, shape) => {
    expect(parseRelease(shape)).toBeNull();
  });
});

describe('targetOfPlatform (MET-019)', () => {
  it('names the target of a Mac, an x86_64 Linux and an aarch64 Linux', () => {
    expect(targetOfPlatform('MacIntel', 'Mozilla/5.0 (Macintosh)')).toBe('macos-aarch64');
    expect(targetOfPlatform('Linux x86_64', 'Mozilla/5.0 (X11; Linux x86_64)')).toBe('linux-x86_64');
    expect(targetOfPlatform('Linux aarch64', 'Mozilla/5.0 (X11; Linux aarch64)')).toBe('linux-aarch64');
  });

  it('gives null when the platform cannot be known', () => {
    expect(targetOfPlatform('Win32', 'Mozilla/5.0 (Windows NT 10.0)')).toBeNull();
    expect(targetOfPlatform('Linux armv8l', 'Mozilla/5.0 (Linux; Android 14)')).toBeNull();
    expect(targetOfPlatform('', '')).toBeNull();
  });
});
