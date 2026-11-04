import { createSubscriber } from 'svelte/reactivity';

/** Whether the operator asked for reduced motion (MOT-040). Reading `reduced` in an effect or template tracks it. */
export interface MotionPreference {
  readonly reduced: boolean;
}

const reducedMotionQuery = '(prefers-reduced-motion: reduce)';
const noPreference: MotionPreference = { reduced: false };
let shared: MotionPreference | undefined;

function follow(media: MediaQueryList): MotionPreference {
  const subscribe = createSubscriber((update) => {
    media.addEventListener('change', update);
    return () => {
      media.removeEventListener('change', update);
    };
  });
  return {
    get reduced() {
      subscribe();
      return media.matches;
    },
  };
}

/**
 * The window's `prefers-reduced-motion` setting, kept current by the query's `change` event
 * (MOT-041). Without a window there is no preference.
 */
export function motionPreference(): MotionPreference {
  if (shared) return shared;
  if (typeof matchMedia !== 'function') return noPreference;
  shared = follow(matchMedia(reducedMotionQuery));
  return shared;
}
