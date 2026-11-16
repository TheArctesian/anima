import { encode } from 'uqr';

/** Blank modules around the code that a phone camera needs to find it. */
export const QUIET_ZONE = 4;

/** A code as one SVG path: `side` modules across, quiet zone included, and the `d` of the dark modules. */
export interface QrDrawing {
  readonly side: number;
  readonly path: string;
}

/** The dark-module matrix of `text`, row by row. The text may be a secret: it goes nowhere but the encoder. */
export function qrMatrix(text: string): readonly (readonly boolean[])[] {
  return encode(text, { border: 0, ecc: 'M' }).data;
}

/** One `M x y h1 v1 h-1 z` square for each dark module, offset by the quiet zone. */
export function qrDrawing(text: string): QrDrawing {
  const matrix = qrMatrix(text);
  const squares: string[] = [];
  matrix.forEach((row, y) => {
    row.forEach((dark, x) => {
      if (dark) squares.push(`M${String(x + QUIET_ZONE)} ${String(y + QUIET_ZONE)}h1v1h-1z`);
    });
  });
  return { side: matrix.length + 2 * QUIET_ZONE, path: squares.join('') };
}
