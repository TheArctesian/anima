// Test support: jsdom has no 2D canvas, and every Guacamole display layer asks for one. This stand-in accepts
// any property write and any method call and draws nothing. No vitest import here.

/** A context that stores what is written to it and answers every other read with a no-op function. */
function blankContext(): object {
  const written = new Map<PropertyKey, unknown>();
  return new Proxy(
    {},
    {
      get: (_target, key) => (written.has(key) ? written.get(key) : () => undefined),
      set: (_target, key, value) => {
        written.set(key, value);
        return true;
      },
    },
  );
}

/** Make `canvas.getContext` give a blank context in jsdom; returns the function that puts the original back. */
export function installBlankCanvas(): () => void {
  const prototype = HTMLCanvasElement.prototype;
  const original = Object.getOwnPropertyDescriptor(prototype, 'getContext');
  Object.defineProperty(prototype, 'getContext', { configurable: true, writable: true, value: blankContext });
  return () => {
    if (original !== undefined) Object.defineProperty(prototype, 'getContext', original);
  };
}
