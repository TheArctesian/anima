import { untrack } from 'svelte';
import { browserClock, skeletonGate, type LoadingClock } from './loading';

/**
 * Whether a loading skeleton shows now (5.7.3, Loading): `loading` must stay true past `threshold-loading` first, and
 * the skeleton then stays `threshold-loading-min`. Call during component init; read `visible` in the template.
 */
export function skeletonDelay(loading: () => boolean, clock: LoadingClock = browserClock): { readonly visible: boolean } {
  let visible = $state(false);
  const gate = skeletonGate(clock, (shown) => {
    visible = shown;
  });

  $effect(() => {
    const busy = loading();
    untrack(() => {
      if (busy) gate.begin();
      else gate.end();
    });
  });

  $effect(() => () => {
    gate.stop();
  });

  return {
    get visible() {
      return visible;
    },
  };
}
