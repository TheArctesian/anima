import { instructionReader, type Instruction } from './instructions';

/**
 * Instructions the player never gets. The internal opcode (empty) is the hub's, not the recording's. `nest` carries
 * instruction text of its own, which the 1.5.5 client would parse past this filter. `audio` would play through the
 * client's default player, and a seek replays every frame before it (playback is silent).
 */
export const DROPPED_OPCODES: ReadonlySet<string> = new Set(['', 'nest', 'audio']);

/** Clean text is gathered in pieces of about this many characters before it joins the Blob's parts. */
const PIECE_CHARS = 262_144;

/**
 * Whether every element is ASCII. 1.5.5's `SessionRecording` finds a frame in its Blob by adding up each element's
 * UTF-16 length, slices the Blob by bytes and reads 256 KiB blocks as text: the three agree only for ASCII. An
 * instruction with another character (a file name, a connection name; never drawing) is dropped, so a frame
 * boundary can never fall inside an element and replay forged text.
 */
function isAscii(instruction: Instruction): boolean {
  for (const value of instruction) {
    for (let at = 0; at < value.length; at += 1) if (value.charCodeAt(at) > 0x7f) return false;
  }
  return true;
}

/** `instruction` written again with canonical lengths, which for ASCII are code points, UTF-16 units and bytes. */
function encode(instruction: Instruction): string {
  return `${instruction.map((value) => `${String(value.length)}.${value}`).join(',')};`;
}

/** The plaintext of a Guacamole recording turned into text the 1.5.5 player reads safely (K4 §11, §12). */
export interface Transcript {
  /** Take the next frame of plaintext; false once the stream is not Guacamole instructions in UTF-8 (and after). */
  receive(bytes: Uint8Array): boolean;
  /** Whether a frame was not Guacamole instructions in UTF-8. */
  unreadable(): boolean;
  /**
   * The whole instructions received so far, cleaned, as one Blob for `Guacamole.SessionRecording`. A last
   * instruction cut by the end of an incomplete or damaged recording is left out.
   */
  blob(): Blob;
}

/** A transcript that has received nothing yet. */
export function openTranscript(): Transcript {
  const decoder = new TextDecoder('utf-8', { fatal: true });
  const reader = instructionReader();
  const parts: string[] = [];
  let piece = '';
  let broken = false;
  const keep = (instructions: readonly Instruction[]) => {
    for (const instruction of instructions) {
      if (DROPPED_OPCODES.has(instruction[0]) || !isAscii(instruction)) continue;
      piece += encode(instruction);
      if (piece.length >= PIECE_CHARS) {
        parts.push(piece);
        piece = '';
      }
    }
  };
  return {
    receive: (bytes) => {
      if (broken) return false;
      let text: string;
      try {
        text = decoder.decode(bytes, { stream: true });
      } catch {
        broken = true;
        return false;
      }
      const instructions = reader.read(text);
      if (instructions === false) broken = true;
      else keep(instructions);
      return !broken;
    },
    unreadable: () => broken,
    blob: () => new Blob([...parts, piece], { type: 'text/plain' }),
  };
}
