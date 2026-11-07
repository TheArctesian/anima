import type { Attachment } from 'svelte/attachments';
import { animate, type Playback } from '../animate';
import { tweenMotion } from '../options';
import { motionPreference, type MotionPreference } from '../preference';

/** The opacity a waiting indicator dims to; it belongs to this role, not to the K5 tokens. */
export const PULSE_DIM = 0.4;

/**
 * The waiting role: a dot that fades down and up while something is awaited. Opacity only, so it moves nothing
 * (MOT-006, MOT-007). Under reduced motion it stays still (MOT-040), and reading the preference here re-runs the
 * attachment when it changes.
 */
export function pulse(preference: MotionPreference = motionPreference()): Attachment<HTMLElement> {
  return (element) => {
    if (preference.reduced) return;
    let stopped = false;
    let running: Playback | undefined;
    const fadeTo = (opacity: number): void => {
      if (stopped) return;
      running = animate(element, { opacity }, tweenMotion('highlight', 'enter', false));
      void running.finished.then(() => {
        fadeTo(opacity === 1 ? PULSE_DIM : 1);
      });
    };
    fadeTo(PULSE_DIM);
    return () => {
      stopped = true;
      running?.stop();
    };
  };
}
