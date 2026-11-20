import { expect, it } from 'vitest';
import { enrollmentCode } from '$lib/link/enrollment-code';
import { qrMatrix, QUIET_ZONE } from './qr';
import { qrText } from './read';

it('decodes the same one-use enrollment QR the desktop client shows', async () => {
  const invite = 'anima-invite.1.dGVzdC1jb2Rl.anima-link.1.test-address';
  const matrix = qrMatrix(invite);
  const scale = 6;
  const width = (matrix.length + 2 * QUIET_ZONE) * scale;
  const data = new Uint8ClampedArray(width * width * 4).fill(255);
  for (let y = 0; y < width; y += 1) {
    for (let x = 0; x < width; x += 1) {
      if (!matrix[Math.floor(y / scale) - QUIET_ZONE]?.[Math.floor(x / scale) - QUIET_ZONE]) continue;
      data.fill(0, (y * width + x) * 4, (y * width + x) * 4 + 3);
    }
  }
  const canvas = { width, height: width, getContext: () => ({ drawImage: () => undefined, getImageData: () => ({ data }) }) };
  const decoded = await qrText(canvas as unknown as HTMLCanvasElement, {} as CanvasImageSource, width, width);
  expect(enrollmentCode(decoded ?? '')).toBe(invite);
  expect(enrollmentCode('https://hub.example.test/sign-in')).toBeNull();
});
