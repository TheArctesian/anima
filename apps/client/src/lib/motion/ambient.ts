/**
 * Whether repeating motion (the "Live" indicator, the terminal cursor) may run: only in a
 * visible window without reduced motion (MOT-011).
 */
export function ambientMotionAllowed(visibility: DocumentVisibilityState, reduced: boolean): boolean {
  return visibility === 'visible' && !reduced;
}
