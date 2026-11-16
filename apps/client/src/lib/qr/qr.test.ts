import { describe, expect, it } from 'vitest';
import { QUIET_ZONE, qrDrawing, qrMatrix } from './qr';

const TOTP = 'otpauth://totp/Anima?secret=JBSWY3DPEHPK3PXP';

describe('QR drawing', () => {
  it('draws the same path for the same text and another for other text', () => {
    expect(qrDrawing('anima://invite/abc').path).toBe(qrDrawing('anima://invite/abc').path);
    expect(qrDrawing('anima://invite/abc').path).not.toBe(qrDrawing('anima://invite/abd').path);
  });

  it('is as wide as the matrix plus a quiet zone of 4 modules on each side', () => {
    const matrix = qrMatrix(TOTP);
    expect(matrix.every((row) => row.length === matrix.length)).toBe(true);
    expect(QUIET_ZONE).toBe(4);
    expect(qrDrawing(TOTP).side).toBe(matrix.length + 8);
  });

  it('keeps every dark module inside the quiet zone', () => {
    const drawing = qrDrawing(TOTP);
    const corners = [...drawing.path.matchAll(/M(\d+) (\d+)h1v1h-1z/g)].flatMap((m) => [Number(m[1]), Number(m[2])]);
    expect(Math.min(...corners)).toBeGreaterThanOrEqual(QUIET_ZONE);
    expect(Math.max(...corners)).toBeLessThan(drawing.side - QUIET_ZONE);
  });

  it('has one square for each dark module, the first at the top-left finder pattern', () => {
    const dark = qrMatrix('hello').flat().filter(Boolean).length;
    const { path } = qrDrawing('hello');
    expect(path.split('z').length - 1).toBe(dark);
    expect(path.startsWith(`M${String(QUIET_ZONE)} ${String(QUIET_ZONE)}h1v1h-1z`)).toBe(true);
  });
});
