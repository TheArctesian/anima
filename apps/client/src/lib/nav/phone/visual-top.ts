/** The part of `VisualViewport` that tells how far the visible area has moved down the page. */
export type VisibleArea = Pick<VisualViewport, 'offsetTop' | 'addEventListener' | 'removeEventListener'>;

/**
 * Keeps `--visual-block-start` on `root` equal to how far the visible area has moved down the page (MOB-011). iOS moves
 * the visible area when the keyboard opens, and a fixed element stays behind; the band over the status bar (`bar.css`)
 * follows this value, so the page never shows under the clock. A window with no visual viewport (a test's) is left alone.
 * Returns the function that stops following.
 */
export function followVisualTop(root: HTMLElement, area: VisibleArea | null | undefined): () => void {
  if (area == null) return () => undefined;
  const update = (): void => {
    root.style.setProperty('--visual-block-start', `${String(Math.max(0, area.offsetTop))}px`);
  };
  update();
  area.addEventListener('resize', update);
  area.addEventListener('scroll', update);
  return () => {
    area.removeEventListener('resize', update);
    area.removeEventListener('scroll', update);
  };
}
