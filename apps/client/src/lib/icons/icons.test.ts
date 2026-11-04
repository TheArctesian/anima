import * as fontkit from 'fontkit';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { glyph, iconCodePoints, type IconName } from './icons';

const propoRegular = fileURLToPath(
  new URL('../../styles/fonts/OpenDyslexicNerdFontPropo-Regular.otf', import.meta.url),
);
const names = Object.keys(iconCodePoints) as IconName[];

function openSingleFont(path: string) {
  const font = fontkit.openSync(path);
  if (!('hasGlyphForCodePoint' in font)) {
    throw new Error(`${path} is a font collection, not a single font`);
  }
  return font;
}

describe('icon table', () => {
  it('UI-014: every icon exists in the bundled Propo font', () => {
    const font = openSingleFont(propoRegular);
    const missing = names.filter((name) => !font.hasGlyphForCodePoint(iconCodePoints[name]));
    expect(missing).toEqual([]);
  });

  it('UI-016: glyph returns the character for each code point', () => {
    for (const name of names) {
      const character = glyph(name);
      expect(character.codePointAt(0), name).toBe(iconCodePoints[name]);
      expect([...character], name).toHaveLength(1);
    }
  });

  it('UI-016: no two icon names share a code point', () => {
    const codePoints = Object.values(iconCodePoints);
    expect(new Set(codePoints).size).toBe(codePoints.length);
  });
});
