import { animate, together, type Animatable, type Playback } from '$lib/motion/animate';
import { springMotion, tweenMotion } from '$lib/motion/options';
import type { MotionPreference } from '$lib/motion/preference';

/** How far above its place the strip starts and ends: half its height, so it slides from the chrome. */
function offset(strip: Animatable): number {
  return -strip.getBoundingClientRect().height / 2;
}

/**
 * The strip comes in when the first tab opens: a fade on `duration-base` and a slide down on
 * `spring-snappy`. Its space is already reserved, so the page does not move during it.
 * `fromHidden` starts it from nothing; otherwise it reverses an exit from where it is (MOT-002).
 * When it arrives its translate is removed, because a translate on the strip would hold a
 * dragged tab's fixed layer inside the strip's clip.
 */
export function enterStrip(strip: Animatable, fromHidden: boolean, preference: MotionPreference): Playback {
  const { reduced } = preference;
  if (fromHidden) animate(strip, { opacity: 0, translate: { y: offset(strip) } }, { type: 'instant' });
  const playback = together([
    animate(strip, { opacity: 1 }, tweenMotion('base', 'enter', reduced)),
    animate(strip, { translate: { y: 0 } }, springMotion('snappy', reduced)),
  ]);
  void playback.finished.then(() => {
    if (strip.style.opacity === '1') strip.style.translate = '';
  });
  return playback;
}

/**
 * The strip leaves after the last tab closes: a fade on `duration-fast` and a slide up by its whole height,
 * so it ends clear of the page that has taken its place and never rests over the page's first row (MOT-007).
 * Reduced: fade.
 */
export function exitStrip(strip: Animatable, preference: MotionPreference): Playback {
  const { reduced } = preference;
  return together([
    animate(strip, { opacity: 0 }, tweenMotion('fast', 'exit', reduced)),
    animate(strip, { translate: { y: -strip.getBoundingClientRect().height } }, springMotion('snappy', reduced)),
  ]);
}
