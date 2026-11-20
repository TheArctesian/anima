/** Decode pixels on this device. Neither the image nor its contents leave the client. */
export async function qrText(canvas: HTMLCanvasElement, source: CanvasImageSource, width: number, height: number): Promise<string | null> {
  if (width === 0 || height === 0) return null;
  const scale = Math.min(1, 1024 / Math.max(width, height));
  const scaledWidth = Math.max(1, Math.round(width * scale));
  const scaledHeight = Math.max(1, Math.round(height * scale));
  if (canvas.width !== scaledWidth) canvas.width = scaledWidth;
  if (canvas.height !== scaledHeight) canvas.height = scaledHeight;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (ctx === null) return null;
  ctx.drawImage(source, 0, 0, canvas.width, canvas.height);
  const { default: jsQR } = await import('jsqr');
  return jsQR(ctx.getImageData(0, 0, canvas.width, canvas.height).data, canvas.width, canvas.height)?.data ?? null;
}
