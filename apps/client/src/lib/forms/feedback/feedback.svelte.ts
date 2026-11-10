import { browserClock, type LoadingClock } from '$lib/motion/loading/loading';
import { duration } from '$lib/motion/tokens';
import { actionFeedback, type ActionState } from './feedback';

/**
 * A button's async feedback for one component (5.7.3, Feedback). `track` runs the work and reads its outcome;
 * `state` goes to the button. Call during component init.
 */
export function useActionFeedback(clock: LoadingClock = browserClock): {
  readonly state: ActionState;
  track<T>(work: Promise<T>, succeeded: (outcome: T) => boolean): Promise<T>;
} {
  let state = $state<ActionState>('idle');
  const feedback = actionFeedback(clock, duration.highlight, (next) => {
    state = next;
  });

  $effect(() => () => {
    feedback.stop();
  });

  return {
    get state() {
      return state;
    },
    async track(work, succeeded) {
      feedback.begin();
      try {
        const outcome = await work;
        if (succeeded(outcome)) feedback.succeed();
        else feedback.fail();
        return outcome;
      } catch (error) {
        feedback.fail();
        throw error;
      }
    },
  };
}
