import type { Attachment } from 'svelte/attachments';
import { motionPreference, type MotionPreference } from '$lib/motion/preference';
import { flip, measure, type Places } from '$lib/motion/roles/flip';

/** Whether two places are the same spot on screen. */
function samePlace(a: DOMRectReadOnly, b: DOMRectReadOnly): boolean {
  return a.left === b.left && a.top === b.top;
}

/**
 * The reorder role (5.7.3) for a keyed list: when `order` changes, the items that moved slide to their new
 * places with `spring-move`. Under reduced motion the order changes at once. Call during component init;
 * attach the returned function to each item with its id.
 */
export function reorderMotion(
  order: () => string,
  preference: MotionPreference = motionPreference(),
): (id: string) => Attachment<HTMLElement> {
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a registry read by effects, never rendered
  const cards = new Map<string, HTMLElement>();
  let before: Places | undefined;

  $effect.pre(() => {
    order();
    before = measure(cards.values());
  });

  /** Slides the items that are not where they were. */
  function play(places: Places): void {
    const moved = [...places].filter(
      ([element, place]) => element.isConnected && !samePlace(place, element.getBoundingClientRect()),
    );
    if (moved.length === 0) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- passed once to flip, not state
    flip(new Map(moved), { preference });
  }

  $effect(() => {
    order();
    const places = before;
    before = undefined;
    if (places !== undefined) play(places);
  });

  /** A leaving item keeps its place until its fade ends; the items after it then close the gap by FLIP. */
  function closeGap(): void {
    const places = measure(cards.values());
    queueMicrotask(() => {
      play(places);
    });
  }

  return (id) => (element) => {
    cards.set(id, element);
    element.addEventListener('outroend', closeGap);
    return () => {
      element.removeEventListener('outroend', closeGap);
      if (cards.get(id) === element) cards.delete(id);
    };
  };
}
