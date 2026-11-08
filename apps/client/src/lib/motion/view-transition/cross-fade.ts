import './cross-fade.css';

/** The part of `Document` a view cross-fade needs; WebKit before Safari 18 and jsdom have no view transitions. */
interface ViewTransitions {
  readonly startViewTransition?: (update: () => void) => Partial<Pick<ViewTransition, 'ready' | 'finished' | 'updateCallbackDone'>> | undefined;
}

/**
 * Runs `change` so the whole view cross-fades from the old look to the new one over `duration-base` (5.7.3 Theme
 * change); the same under reduced motion, because only opacity animates (MOT-006, MOT-040). The browser snapshots the
 * page, calls `change` on its next frame and fades the snapshot into the live page; a second call during the fade
 * ends the first at its new look, and `change` always runs, even when the browser skips the fade. Where view
 * transitions are not available, `change` runs at once. Nothing waits for the fade (MOT-042).
 */
export function crossFadeView(change: () => void, view: ViewTransitions = document): void {
  if (typeof view.startViewTransition !== 'function') {
    change();
    return;
  }
  // MOT-010: the whole page is captured. Only Settings calls this, and while Settings shows no session pane is on screen
  // (a route change deselects sessions), so no terminal or remote display fades; F-5 keeps session colours anyway.
  const transition = view.startViewTransition(change);
  // A skipped transition (a second change, a hidden page) is not an error, and `change` has run: mark them handled.
  for (const outcome of [transition?.ready, transition?.finished, transition?.updateCallbackDone]) {
    outcome?.catch(() => undefined);
  }
}
