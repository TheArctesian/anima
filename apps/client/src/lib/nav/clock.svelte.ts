import { createSubscriber } from 'svelte/reactivity';

/** The time as Svelte state: while an effect or the template reads `current`, it refreshes every `intervalMs`. */
export function createClock(intervalMs: number): { readonly current: Date } {
  let latest = Date.now();
  const tick = createSubscriber((update) => {
    latest = Date.now();
    const timer = setInterval(() => {
      latest = Date.now();
      update();
    }, intervalMs);
    return () => {
      clearInterval(timer);
    };
  });
  return {
    get current() {
      tick();
      // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a read-only snapshot; `tick()` is the reactive part
      return new Date(latest);
    },
  };
}
