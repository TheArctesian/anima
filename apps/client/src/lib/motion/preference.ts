import { createSubscriber } from 'svelte/reactivity';

/** Whether the operator asked for reduced motion (MOT-040). Reading `reduced` in an effect or template tracks it. */
export interface MotionPreference {
  readonly reduced: boolean;
}

const reducedMotionQuery = '(prefers-reduced-motion: reduce)';
const noPreference: MotionPreference = { reduced: false };
let shared: MotionPreference | undefined;

/**
 * The Settings choice wins over the OS (MOT-043): `data-motion="reduce"` is reduced and `"full"` is not, whatever
 * the OS says; without the attribute (System) the OS decides. The generated `tokens.css` reads the same attribute.
 */
function reduces(choice: string | undefined, systemReduces: boolean): boolean {
  if (choice === 'reduce') return true;
  if (choice === 'full') return false;
  return systemReduces;
}

function follow(root: HTMLElement, media: MediaQueryList | undefined): MotionPreference {
  const subscribe = createSubscriber((update) => {
    media?.addEventListener('change', update);
    const choice = new MutationObserver(update);
    choice.observe(root, { attributes: true, attributeFilter: ['data-motion'] });
    return () => {
      media?.removeEventListener('change', update);
      choice.disconnect();
    };
  });
  return {
    get reduced() {
      subscribe();
      return reduces(root.dataset['motion'], media?.matches ?? false);
    },
  };
}

/**
 * The operator's reduced-motion setting: the Settings choice on `<html data-motion>`, else the window's
 * `prefers-reduced-motion`, kept current by the attribute and the query's `change` event, so a change applies
 * without a reload (MOT-041, MOT-043). Without a document there is no preference.
 */
export function motionPreference(): MotionPreference {
  if (shared) return shared;
  if (typeof document === 'undefined') return noPreference;
  shared = follow(document.documentElement, typeof matchMedia === 'function' ? matchMedia(reducedMotionQuery) : undefined);
  return shared;
}
