import type { Attachment } from 'svelte/attachments';
import { animate } from '../animate';
import { springMotion, tweenMotion } from '../options';
import { motionPreference, type MotionPreference } from '../preference';
import { roleScale } from './scales';

const releases = ['pointerup', 'pointercancel', 'pointerleave'] as const;

/**
 * The press role (5.7.3): scale down on pointer-down, not on click (MOT-001), and spring back
 * on release. Under reduced motion nothing moves; the colour change in CSS is the feedback.
 */
export function press(preference: MotionPreference = motionPreference()): Attachment<HTMLElement> {
  return (element) => {
    let pressed = false;
    const down = (event: PointerEvent) => {
      if (event.button !== 0 || preference.reduced) return;
      pressed = true;
      animate(element, { scale: roleScale.press }, tweenMotion('press', 'enter', preference.reduced));
    };
    const release = () => {
      if (!pressed) return;
      pressed = false;
      animate(element, { scale: 1 }, springMotion('default', preference.reduced));
    };
    element.addEventListener('pointerdown', down);
    for (const name of releases) element.addEventListener(name, release);
    return () => {
      element.removeEventListener('pointerdown', down);
      for (const name of releases) element.removeEventListener(name, release);
    };
  };
}
