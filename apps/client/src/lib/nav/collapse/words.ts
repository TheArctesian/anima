import { animate, together, type Playback } from '$lib/motion/animate';
import { tweenMotion } from '$lib/motion/options';
import { motionPreference, type MotionPreference } from '$lib/motion/preference';

/**
 * When the bar expands its words fade in over `duration-base` with `ease-enter`, so they arrive with the page's glide
 * rather than on one frame (NAV-006). Opacity only: the width itself switches at once (MOT-006).
 */
export function fadeInWords(bar: ParentNode, preference: MotionPreference = motionPreference()): Playback {
  const motion = tweenMotion('base', 'enter', preference.reduced);
  return together(
    [...bar.querySelectorAll<HTMLElement>('.words')].map((words) => {
      animate(words, { opacity: 0 }, { type: 'instant' });
      return animate(words, { opacity: 1 }, motion);
    }),
  );
}
