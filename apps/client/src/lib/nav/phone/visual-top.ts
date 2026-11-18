/** The part of `VisualViewport` that tells where the visible area is on the page and how tall it is. */
export type VisibleArea = Pick<VisualViewport, 'offsetTop' | 'height' | 'addEventListener' | 'removeEventListener'>;

/**
 * Keeps `--visual-block-start` on `root` equal to how far the visible area has moved down the page (MOB-011). iOS moves
 * the visible area when the keyboard opens, and a fixed element stays behind; the band over the status bar (`bar.css`)
 * follows this value, so the page never shows under the clock. `--visual-block-size` is the visible area's height, which
 * the keyboard makes shorter; a session on a phone fills it (`session.css`). A window with no visual viewport (a test's) is left alone.
 * Returns the function that stops following.
 */
export function followVisualTop(root: HTMLElement, area: VisibleArea | null | undefined): () => void {
  if (area == null) return () => undefined;
  const update = (): void => {
    root.style.setProperty('--visual-block-start', `${String(Math.max(0, area.offsetTop))}px`);
    root.style.setProperty('--visual-block-size', `${String(area.height)}px`);
  };
  update();
  area.addEventListener('resize', update);
  area.addEventListener('scroll', update);
  return () => {
    area.removeEventListener('resize', update);
    area.removeEventListener('scroll', update);
  };
}

