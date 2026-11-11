import { untrack } from 'svelte';
import type { Attachment } from 'svelte/attachments';
import { animate, type Animatable, type Playback } from '../animate';
import { springMotion } from '../options';
import { motionPreference, type MotionPreference } from '../preference';

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

/** The look a copy needs outside its page: what the page's own rules gave the element. */
const LOOK = [
  'font',
  'letter-spacing',
  'color',
  'background',
  'border',
  'border-radius',
  'padding',
  'display',
  'align-items',
  'justify-content',
  'gap',
  'white-space',
  'overflow',
  'text-overflow',
] as const;

/**
 * A copy of `element` fixed at `place`, above both pages, so it flies at full strength while the pages cross-fade
 * beneath it (5.7.3, card to detail). It carries the shared key, so a page that mounts mid-flight departs from it.
 */
function copyAt(element: HTMLElement, place: DOMRectReadOnly): HTMLElement {
  const copy = element.cloneNode(true) as HTMLElement;
  const style = getComputedStyle(element);
  for (const property of LOOK) copy.style.setProperty(property, style.getPropertyValue(property));
  copy.removeAttribute('id');
  copy.setAttribute('aria-hidden', 'true');
  copy.inert = true;
  copy.dataset.flight = '';
  Object.assign(copy.style, {
    position: 'fixed',
    left: `${String(place.left)}px`,
    top: `${String(place.top)}px`,
    width: `${String(place.width)}px`,
    height: `${String(place.height)}px`,
    margin: '0',
    boxSizing: 'border-box',
    pointerEvents: 'none',
    zIndex: 'var(--layer-popover)',
    opacity: '1',
    translate: '',
    scale: '',
  });
  document.body.append(copy);
  return copy;
}

function ratio(before: number, after: number): number {
  return after === 0 ? 1 : before / after;
}

/**
 * The card to detail role (5.7.3): a copy of the item flies from where the same `key` is on the
 * outgoing page to its place on this one, with `spring-move`, by `translate` and `scale` only
 * (MOT-006), above both pages at full opacity; the item itself shows when the copy lands. The outgoing element starts from where it is on screen, even mid-flight
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
      flight.land();
    };
  };
}

interface Flight {
  readonly playback: Playback;
  /** Stops the copy, removes it and shows both ends; safe to call twice. */
  readonly land: () => void;
}

/** Flies `copy` from `from` to its own place, centre to centre, by `translate` and `scale` on `spring-move`. */
function fly(copy: Animatable, from: DOMRectReadOnly, to: DOMRectReadOnly, preference: MotionPreference): Playback {
  const translate = {
    x: from.left + from.width / 2 - (to.left + to.width / 2),
    y: from.top + from.height / 2 - (to.top + to.height / 2),
  };
  const scale = { x: ratio(from.width, to.width), y: ratio(from.height, to.height) };
  animate(copy, { translate, scale }, { type: 'instant' });
  return animate(copy, { translate: { x: 0, y: 0 }, scale: 1 }, springMotion('move', preference.reduced));
}

function depart(element: HTMLElement, key: string, preference: MotionPreference): Flight | undefined {
  if (preference.reduced) return undefined;
  const outgoing = counterpart(element, key);
  if (outgoing === undefined) return undefined;
  const from = outgoing.getBoundingClientRect();
  const to = element.getBoundingClientRect();
  const showOutgoing = hide(outgoing);
  const showArriving = hide(element);
  // A copy that was itself in flight is the outgoing end; it goes once the new copy has taken its place.
  if (outgoing.dataset.flight !== undefined) outgoing.remove();
  const copy = copyAt(element, to);
  const playback = fly(copy, from, to, preference);
  const land = () => {
    playback.stop();
    copy.remove();
    showOutgoing();
    showArriving();
  };
  playback.finished.then(land, land);
  return { playback, land };
}
