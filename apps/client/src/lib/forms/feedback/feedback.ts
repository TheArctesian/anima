import type { LoadingClock } from '$lib/motion/loading/loading';
import { threshold } from '$lib/motion/tokens';

/** What a button that starts hub work shows: its own label, a pending label, or that the work is done. */
export type ActionState = 'idle' | 'pending' | 'done';

export interface ActionFeedback {
  /** The work started; the pending label shows only if it is still running after `threshold-loading`. */
  begin(): void;
  /** The work succeeded; the done state shows for `holdMs`, then the label returns. */
  succeed(): void;
  /** The work failed; the label returns at once and the caller says why beside it. */
  fail(): void;
  /** Cancel every wait, for a button that is going away. */
  stop(): void;
}

/**
 * Async feedback on a button (5.7.3, Feedback): no pending label for quick work, so it never flashes, and a done
 * state that holds long enough to read. `onChange` hears each change of state.
 */
export function actionFeedback(clock: LoadingClock, holdMs: number, onChange: (state: ActionState) => void): ActionFeedback {
  let cancel: (() => void) | undefined;
  const clear = () => {
    cancel?.();
    cancel = undefined;
  };
  const settle = (state: ActionState) => {
    clear();
    onChange(state);
  };

  return {
    begin: () => {
      clear();
      cancel = clock.after(threshold.loading, () => {
        cancel = undefined;
        onChange('pending');
      });
    },
    succeed: () => {
      settle('done');
      cancel = clock.after(holdMs, () => {
        cancel = undefined;
        onChange('idle');
      });
    },
    fail: () => {
      settle('idle');
    },
    stop: clear,
  };
}
