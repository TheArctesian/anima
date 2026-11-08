import './cross-fade.css';

/** The part of `Document` a view cross-fade needs; WebKit before Safari 18 and jsdom have no view transitions. */
interface ViewTransitions {
  readonly startViewTransition?: (update: () => void) => unknown;
}

/**
 * Runs `change` so the whole view cross-fades from the old look to the new one over `duration-base` (5.7.3 Theme
 * change); the same under reduced motion, because only opacity animates (MOT-006, MOT-040). The browser snapshots the
 * page, calls `change` on its next frame and fades the snapshot into the live page; a second call during the fade
 * ends the first at its new look, and `change` always runs, even when the browser skips the fade. Where view
 * transitions are not available, `change` runs at once. Nothing waits for the fade (MOT-042).
 */
export function crossFadeView(change: () => void, view: ViewTransitions = document): void {
  if (typeof view.startViewTransition === 'function') view.startViewTransition(change);
  else change();
}
