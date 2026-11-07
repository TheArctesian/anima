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
