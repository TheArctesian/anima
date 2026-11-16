import { describe, expect, it } from "vitest";
import { QUIET_ZONE, type QrDrawing, qrDrawing, qrMatrix } from "./qr";

const TOTP = "otpauth://totp/Anima?secret=JBSWY3DPEHPK3PXP";

function drawn(text: string): QrDrawing {
  const drawing = qrDrawing(text);
  if (drawing === null) throw new Error("no drawing");
  return drawing;
}

describe("QR drawing", () => {
  it("draws the same path for the same text and another for other text", () => {
    expect(qrDrawing("anima://invite/abc")?.path).toBe(
      qrDrawing("anima://invite/abc")?.path,
    );
    expect(qrDrawing("anima://invite/abc")?.path).not.toBe(
      qrDrawing("anima://invite/abd")?.path,
    );
  });

  it("is as wide as the matrix plus a quiet zone of 4 modules on each side", () => {
    const matrix = qrMatrix(TOTP);
    expect(matrix.every((row) => row.length === matrix.length)).toBe(true);
    expect(QUIET_ZONE).toBe(4);
    expect(drawn(TOTP).side).toBe(matrix.length + 8);
  });

  it("keeps every dark module inside the quiet zone", () => {
    const drawing = drawn(TOTP);
    const corners = [...drawing.path.matchAll(/M(\d+) (\d+)h1v1h-1z/g)].flatMap(
      (m) => [Number(m[1]), Number(m[2])],
    );
    expect(Math.min(...corners)).toBeGreaterThanOrEqual(QUIET_ZONE);
    expect(Math.max(...corners)).toBeLessThan(drawing.side - QUIET_ZONE);
  });

  it("has one square for each dark module, the first at the top-left finder pattern", () => {
    const dark = qrMatrix("hello").flat().filter(Boolean).length;
    const { path } = drawn("hello");
    expect(path.split("z").length - 1).toBe(dark);
    expect(
      path.startsWith(`M${String(QUIET_ZONE)} ${String(QUIET_ZONE)}h1v1h-1z`),
    ).toBe(true);
  });

  it("uses error correction level M, so that a printed code survives a smudge", () => {
    // Fifteen bytes fit version 1 at level L (21 modules) and need version 2 at level M (25).
    expect(qrMatrix("x".repeat(15)).length).toBe(25);
  });

  it("draws nothing for a text too long for any code", () => {
    expect(qrDrawing("x".repeat(5000))).toBeNull();
  });
});
