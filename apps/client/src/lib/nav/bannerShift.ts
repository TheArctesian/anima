import type { Animatable } from '$lib/motion/animate';
import { motionPreference, type MotionPreference } from '$lib/motion/preference';
import { flip, measure, type Places } from '$lib/motion/roles/flip';

/**
 * The attribute on the root element while the unreachable strip shows. The stylesheets of the bar and of the page read it
 * to make room for the strip below `bp-compact`.
 */
const ATTRIBUTE = 'data-hub-banner';

/** Marks a container whose children make room for the strip; each child moves by FLIP when the strip comes or goes. */
const SHIFTS_WITH_BANNER = 'data-shifts-with-banner';

/** Marks the child that holds the fixed strip: a `translate` on it would become the strip's containing block. */
const HOLDS_BANNER = 'data-holds-banner';

function shifting(root: HTMLElement): Animatable[] {
  const found: Animatable[] = [];
  for (const container of root.querySelectorAll(`[${SHIFTS_WITH_BANNER}]`)) {
    for (const child of container.children) {
      if (child instanceof HTMLElement && !child.hasAttribute(HOLDS_BANNER)) found.push(child);
    }
  }
  return found;
}

/** The measured elements whose place changed; an element that stays put gets no `translate` (it would hold fixed children). */
function moved(first: Places): Places {
  return new Map(
    [...first].filter(([element, before]) => {
      const after = element.getBoundingClientRect();
      return after.top !== before.top || after.left !== before.left;
    }),
  );
}

/** A `translate` at rest is dropped, so a modal or a session pane under the page is not positioned against it. */
function settle(places: Places): void {
  for (const [element] of places) {
    const [x = 0, y = 0] = element.style.translate.split(/\s+/).map((part) => Number.parseFloat(part));
    if (Math.abs(x) < 1 && Math.abs(y) < 1) element.style.translate = '';
  }
}

/**
 * Sets `data-hub-banner` on `root` while the strip shows. The page's room for the strip changes at once; what sits in the
 * marked containers moves from where it was to where it is on `spring-default`, and a change mid-flight starts from the
 * place on screen (MOT-002, MOT-007). The first call sets the state with no motion, since nothing was on screen before.
 */
export function watchHubBanner(root: HTMLElement, preference: MotionPreference = motionPreference()) {
  let known = false;
  return {
    show(shown: boolean): void {
      if (!known) {
        known = true;
        root.toggleAttribute(ATTRIBUTE, shown);
        return;
      }
      if (root.hasAttribute(ATTRIBUTE) === shown) return;
      const first = measure(shifting(root));
      root.toggleAttribute(ATTRIBUTE, shown);
      const places = moved(first);
      void flip(places, { spring: 'default', preference }).finished.then(() => {
        settle(places);
      });
    },
    stop(): void {
      root.removeAttribute(ATTRIBUTE);
    },
  };
}
