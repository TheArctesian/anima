/** Where the pages were scrolled when a session tab covered them (WP-14 E-2). */
export interface CoveredScroll {
  readonly path: string;
  readonly y: number;
}

/**
 * The scroll position to give the pages when they show again at `path`. A covered page has no height, so the
 * document loses its scroll position; the same page gets back the one it had when covered, a different page keeps
 * the position navigation gives it.
 */
export function restoredScroll(covered: CoveredScroll | null, path: string): number | undefined {
  return covered?.path === path ? covered.y : undefined;
}
