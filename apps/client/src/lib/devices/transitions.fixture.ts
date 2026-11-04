/** jsdom has no Web Animations: a Svelte transition ends at once. Call once at the top of a component test. */
export function endTransitionsAtOnce(): void {
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: function animate() {
      const playback = {
        onfinish: null as (() => void) | null,
        cancel: () => undefined,
        finish: () => undefined,
        currentTime: 0,
      };
      queueMicrotask(() => playback.onfinish?.());
      return playback;
    },
  });
}
