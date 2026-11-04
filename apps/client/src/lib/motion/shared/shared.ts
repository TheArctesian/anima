import { untrack } from 'svelte';
import type { Attachment } from 'svelte/attachments';
import type { Animatable, Playback } from '../animate';
import { motionPreference, type MotionPreference } from '../preference';
import { flip } from '../roles/flip';

export interface SharedOptions {
  readonly preference?: MotionPreference;
}

/**
 * The other element on the page with `key`: the last one in document order. The layout's
 * keyed block mounts each new page after the pages still fading out, so the last other match
 * is the most recently mounted outgoing page.
 */
function counterpart(element: HTMLElement, key: string): HTMLElement | undefined {
  const matches = [...document.querySelectorAll<HTMLElement>('[data-shared]')].filter(
    (other) => other !== element && other.dataset.shared === key,
  );
  return matches.at(-1);
}

/**
 * Hides `element` until the returned function runs, so the item is not seen twice while its
 * page fades out; the function puts back the inline opacity it found and is safe to call twice.
 */
function hide(element: Animatable): () => void {
  const before = element.style.opacity;
  element.style.opacity = '0';
  let shown = false;
  return () => {
    if (shown) return;
    shown = true;
    element.style.opacity = before;
  };
}

/**
 * The card to detail role (5.7.3): the item moves from where the same `key` is on the
 * outgoing page to its place on this one, with `spring-move`, by `translate` and `scale` only
 * (MOT-006). The outgoing element starts from where it is on screen, even mid-flight
 * (MOT-002). Under reduced motion nothing moves; the page cross-fade is the reduced form
 * (MOT-040, MOT-041). Detaching mid-flight stops the flight and clears the styles it wrote.
 *
 * A key must be unique on a page, so each page has one counterpart for it. The element must
 * keep the default `transform-origin` (its centre), because the flight runs centre to centre,
 * and must not be a non-replaced inline element, to which `translate` and `scale` do not apply.
 */
export function sharedItem(key: string, options: SharedOptions = {}): Attachment<HTMLElement> {
  const { preference = motionPreference() } = options;
  return (element) => {
    element.dataset.shared = key;
    // The preference is read once per attach; a later change must not re-run the flight.
    const flight = untrack(() => depart(element, key, preference));
    return () => {
      delete element.dataset.shared;
      if (flight === undefined) return;
      flight.playback.stop();
      flight.show();
      element.style.translate = '';
      element.style.scale = '';
    };
  };
}

interface Flight {
  readonly playback: Playback;
  readonly show: () => void;
}

function depart(element: HTMLElement, key: string, preference: MotionPreference): Flight | undefined {
  if (preference.reduced) return undefined;
  const outgoing = counterpart(element, key);
  if (outgoing === undefined) return undefined;
  const from = outgoing.getBoundingClientRect();
  const show = hide(outgoing);
  const playback = flip(new Map([[element, from]]), { spring: 'move', scale: true, preference });
  playback.finished.then(show, show);
  return { playback, show };
}
