/**
 * The longest instruction the reader holds, in code points, length prefixes and separators included. guacd writes
 * far shorter ones (a `blob` carries at most 6048 bytes as base64), so a longer one is a fault, not a wait.
 */
export const INSTRUCTION_MAX_POINTS = 65_536;

/** A length prefix of more digits than `INSTRUCTION_MAX_POINTS` has is a fault before its value is read. */
const LENGTH_MAX_DIGITS = String(INSTRUCTION_MAX_POINTS).length;

/** One Guacamole instruction: the opcode, then its arguments. */
export type Instruction = readonly [opcode: string, ...args: string[]];

/** The whole instructions of one read, or `false` once the text broke the protocol (and every read after). */
export type ReadResult = readonly Instruction[] | false;

/** Reads Guacamole instruction text in pieces cut anywhere (K4 §11). */
export interface InstructionReader {
  /** Read `text` after what an earlier read left over; a cut last instruction waits for the next read. */
  read(text: string): ReadResult;
  /** How many UTF-16 units of a cut instruction wait for the next read. */
  waiting(): number;
}

/** Where an element ends: the index after its terminator, or `wait` (the text stops first), or `fault`. */
type Element = { readonly value: string; readonly next: number; readonly last: boolean; readonly points: number } | 'wait' | 'fault';

function isDigit(code: number): boolean {
  return code >= 0x30 && code <= 0x39;
}

/** The index after `count` code points from `start`, or -1 when `text` ends first. */
function skipPoints(text: string, start: number, count: number): number {
  let at = start;
  for (let seen = 0; seen < count; seen += 1) {
    if (at >= text.length) return -1;
    const point = text.codePointAt(at) ?? 0;
    at += point > 0xff_ff ? 2 : 1;
  }
  return at;
}

/**
 * One element at `start`: `<length>.<value>` and `,` or `;`, the length in code points as guacd and the hub count
 * it (K4 §4). The declared length is checked against `budget` before the value is waited for.
 */
function element(text: string, start: number, budget: number): Element {
  let at = start;
  while (at < text.length && isDigit(text.charCodeAt(at))) at += 1;
  const digits = at - start;
  if (digits > LENGTH_MAX_DIGITS) return 'fault';
  if (at >= text.length) return 'wait';
  if (digits === 0 || text[at] !== '.') return 'fault';
  const length = Number(text.slice(start, at));
  const points = digits + length + 2;
  if (points > budget) return 'fault';
  const end = skipPoints(text, at + 1, length);
  if (end === -1 || end >= text.length) return 'wait';
  const terminator = text[end];
  if (terminator !== ',' && terminator !== ';') return 'fault';
  return { value: text.slice(at + 1, end), next: end + 1, last: terminator === ';', points };
}

/** One instruction at `start`, with the index after it; `wait` when the text stops inside it. */
function instruction(text: string, start: number): { readonly value: Instruction; readonly next: number } | 'wait' | 'fault' {
  const elements: string[] = [];
  let at = start;
  let budget = INSTRUCTION_MAX_POINTS;
  for (;;) {
    const found = element(text, at, budget);
    if (typeof found === 'string') return found;
    elements.push(found.value);
    budget -= found.points;
    at = found.next;
    if (found.last) {
      const [opcode = '', ...args] = elements;
      return { value: [opcode, ...args], next: at };
    }
  }
}

/**
 * A reader that never trusts a length: each one is bounded before the text it names is waited for, a cut
 * instruction is held (at most `INSTRUCTION_MAX_POINTS` code points), and anything that is not an instruction is
 * a fault that stays.
 */
export function instructionReader(): InstructionReader {
  let rest = '';
  let broken = false;
  return {
    read: (text) => {
      if (broken) return false;
      rest += text;
      const found: Instruction[] = [];
      let at = 0;
      for (;;) {
        const next = at < rest.length ? instruction(rest, at) : 'wait';
        if (next === 'fault') {
          broken = true;
          rest = '';
          return false;
        }
        if (next === 'wait') break;
        found.push(next.value);
        at = next.next;
      }
      rest = rest.slice(at);
      return found;
    },
    waiting: () => rest.length,
  };
}
