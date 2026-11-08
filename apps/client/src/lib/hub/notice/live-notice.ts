import type { Animatable } from '$lib/motion/animate';
import type { Places } from '$lib/motion/roles/flip';

/** The elements after `anchor` in its parent: what the notice's height pushes down or lets up. */
export function followers(anchor: Element): Animatable[] {
  const found: Animatable[] = [];
  for (let next = anchor.nextElementSibling; next; next = next.nextElementSibling) {
    if (next instanceof HTMLElement) found.push(next);
  }
  return found;
}

/**
 * The measured elements whose place changed. The others get no `translate`, which would make an element the containing
 * block of its fixed children.
 */
export function moved(first: Places): Places {
  return new Map(
    [...first].filter(([element, before]) => {
      const after = element.getBoundingClientRect();
      return after.top !== before.top || after.left !== before.left;
    }),
  );
}
