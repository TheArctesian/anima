import { describe, expect, it } from 'vitest';
import { encodeInstruction } from '../fake/socket.fixture';
import { INSTRUCTION_MAX_POINTS, instructionReader, type Instruction } from './instructions';

/** A small seeded generator (mulberry32), so every property run is the same run. */
function random(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state + 0x6d_2b_79_f5) | 0;
    let mixed = Math.imul(state ^ (state >>> 15), 1 | state);
    mixed = (mixed + Math.imul(mixed ^ (mixed >>> 7), 61 | mixed)) ^ mixed;
    return ((mixed ^ (mixed >>> 14)) >>> 0) / 4_294_967_296;
  };
}

const ALPHABET = ['a', 'Z', '0', '9', '.', ',', ';', ' ', 'é', '€', '😀', '\u0000', '\n'];

function word(next: () => number, longest: number): string {
  const length = Math.floor(next() * longest);
  return Array.from({ length }, () => ALPHABET[Math.floor(next() * ALPHABET.length)] ?? 'a').join('');
}

function instructions(next: () => number): Instruction[] {
  return Array.from({ length: 1 + Math.floor(next() * 8) }, () => {
    const args = Array.from({ length: Math.floor(next() * 5) }, () => word(next, 12));
    return [word(next, 6), ...args] as const;
  });
}

/** `text` cut at random UTF-16 indexes (a cut may fall inside a surrogate pair). */
function pieces(text: string, next: () => number): string[] {
  const cut: string[] = [];
  let at = 0;
  while (at < text.length) {
    const size = 1 + Math.floor(next() * 9);
    cut.push(text.slice(at, at + size));
    at += size;
  }
  return cut;
}

function readAll(texts: readonly string[]) {
  const reader = instructionReader();
  const found: Instruction[] = [];
  for (const text of texts) {
    const result = reader.read(text);
    if (result === false) return { found, fault: true, waiting: reader.waiting() };
    found.push(...result);
  }
  return { found, fault: false, waiting: reader.waiting() };
}

describe('the Guacamole instruction reader (K4 §11, §12)', () => {
  it('K4 §4: element lengths count code points, so a character outside the BMP counts once', () => {
    expect(readAll(['4.name,2.😀a;4.sync,1.0;'])).toEqual({ found: [['name', '😀a'], ['sync', '0']], fault: false, waiting: 0 });
  });

  it('K4 §12: an instruction cut across frames waits for the rest; a last cut one stays waiting', () => {
    expect(readAll(['4.sy', 'nc,4.1', '000;4.si'])).toEqual({ found: [['sync', '1000']], fault: false, waiting: 4 });
  });

  it('K4 §12: property — any cut of encoded instructions reads back the same instructions (500 seeded runs)', () => {
    for (let seed = 1; seed <= 500; seed += 1) {
      const next = random(seed);
      const sent = instructions(next);
      const text = sent.map(([opcode, ...args]) => encodeInstruction(opcode, ...args)).join('');
      expect(readAll(pieces(text, next)), `seed ${String(seed)}`).toEqual({ found: sent, fault: false, waiting: 0 });
    }
  });

  it.each([
    ['a length that is not digits', 'x.size;'],
    ['a missing dot', '4,size;'],
    ['a wrong terminator', '4.size:'],
    ['an empty length', '.;'],
    ['a sign', '+4.size;'],
    ['a length with more digits than the limit', '0000001.a;'],
  ])('a fault: %s', (_name, text) => {
    expect(readAll([text]).fault).toBe(true);
  });

  it('a declared length that would take an instruction over INSTRUCTION_MAX_POINTS is a fault at once, before its text is waited for', () => {
    // "4.blob," takes 7 points; "<5 digits>.<L>," takes 7 + L, so L = MAX - 14 is the most that fits.
    expect(readAll([`${String(INSTRUCTION_MAX_POINTS)}.`]).fault).toBe(true);
    expect(readAll([`4.blob,${String(INSTRUCTION_MAX_POINTS - 13)}.`]).fault).toBe(true);
    expect(readAll([`4.blob,${String(INSTRUCTION_MAX_POINTS - 14)}.`])).toMatchObject({ fault: false, found: [] });
  });

  it('many empty elements count towards the limit too', () => {
    expect(readAll(['0.,'.repeat(INSTRUCTION_MAX_POINTS / 3 + 1)]).fault).toBe(true);
  });

  it('a fault stays: every later read is a fault, even of good text', () => {
    const reader = instructionReader();
    expect(reader.read('4.sync,1.0;x')).toBe(false);
    expect(reader.read('4.sync,1.0;')).toBe(false);
    expect(reader.waiting()).toBe(0);
  });

  it('property — random text never throws and never holds more than the limit allows (500 seeded runs)', () => {
    for (let seed = 1; seed <= 500; seed += 1) {
      const next = random(seed);
      const text = Array.from({ length: 200 }, () => (next() < 0.3 ? String(Math.floor(next() * 99_999)) : word(next, 4))).join('');
      const result = readAll(pieces(text, next));
      expect(result.waiting).toBeLessThanOrEqual(INSTRUCTION_MAX_POINTS * 2);
    }
  });
});
