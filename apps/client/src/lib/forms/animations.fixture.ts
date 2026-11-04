/** What Svelte's transitions asked the browser to play; jsdom has no Web Animations. */
export const played: { element: Element; keyframes: Keyframe[]; options: KeyframeAnimationOptions }[] = [];

/**
 * Installs an `Element.animate` that records the call and finishes at once, so that an outro
 * removes its element in the test without a timer.
 */
export function installAnimations(): void {
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value(this: Element, keyframes: Keyframe[], options: KeyframeAnimationOptions) {
      played.push({ element: this, keyframes, options });
      const animation = {
        playState: 'running',
        currentTime: 0,
        cancel: () => undefined,
        pause: () => undefined,
        set onfinish(done: () => void) {
          queueMicrotask(done);
        },
      };
      return animation;
    },
  });
}
