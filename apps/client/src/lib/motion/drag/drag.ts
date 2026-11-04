import type { Attachment } from 'svelte/attachments';
import { velocityOf, type Sample } from './velocity';

/** Where the dragged item is, and how far it has come from where it was grabbed (px along the axis). */
export interface Dragged {
  readonly position: number;
  readonly delta: number;
}

/** The end of a drag. `velocity` is in px/s along the axis; the caller picks where the item comes to rest (MOT-031, MOT-032). */
export interface Released extends Dragged {
  readonly velocity: number;
}

export interface DragOptions {
  readonly axis: 'x' | 'y';
  /** Where the item is now, in the same client coordinates as the pointer; read at the grab. */
  readonly origin: () => number;
  readonly onGrab?: () => void;
  readonly onMove: (dragged: Dragged) => void;
  /** Called on pointer up and on pointer cancel. */
  readonly onRelease: (released: Released) => void;
}

interface Grip {
  readonly pointerId: number;
  /** Pointer minus item at the grab, kept for the whole drag (MOT-030). */
  readonly offset: number;
  readonly grabbed: number;
  readonly samples: Sample[];
}

/**
 * Direct manipulation along one axis (5.7.4): the item follows the pointer 1:1 and keeps the offset from the grab point
 * (MOT-030), with the pointer captured. It reports positions and the release velocity and moves nothing itself.
 */
export function drag(options: DragOptions): Attachment<HTMLElement> {
  const { axis, origin, onGrab, onMove, onRelease } = options;
  const coordinate = (event: PointerEvent) => (axis === 'x' ? event.clientX : event.clientY);

  return (element) => {
    let grip: Grip | undefined;

    const describe = (held: Grip): Dragged => {
      const position = (held.samples.at(-1)?.value ?? 0) - held.offset;
      return { position, delta: position - held.grabbed };
    };
    const own = (event: PointerEvent): Grip | undefined => (grip?.pointerId === event.pointerId ? grip : undefined);

    const down = (event: PointerEvent) => {
      if (event.button !== 0 || grip !== undefined) return;
      element.setPointerCapture(event.pointerId);
      const pointer = coordinate(event);
      const grabbed = origin();
      grip = {
        pointerId: event.pointerId,
        offset: pointer - grabbed,
        grabbed,
        samples: [{ at: event.timeStamp, value: pointer }],
      };
      onGrab?.();
    };
    const move = (event: PointerEvent) => {
      const held = own(event);
      if (held === undefined) return;
      held.samples.push({ at: event.timeStamp, value: coordinate(event) });
      onMove(describe(held));
    };
    const end = (event: PointerEvent) => {
      const held = own(event);
      if (held === undefined) return;
      // A cancelled pointer has no position of its own; the last reading stands.
      if (event.type === 'pointerup') held.samples.push({ at: event.timeStamp, value: coordinate(event) });
      grip = undefined;
      onRelease({ ...describe(held), velocity: velocityOf(held.samples, event.timeStamp) });
    };

    element.addEventListener('pointerdown', down);
    element.addEventListener('pointermove', move);
    element.addEventListener('pointerup', end);
    element.addEventListener('pointercancel', end);
    return () => {
      element.removeEventListener('pointerdown', down);
      element.removeEventListener('pointermove', move);
      element.removeEventListener('pointerup', end);
      element.removeEventListener('pointercancel', end);
    };
  };
}
