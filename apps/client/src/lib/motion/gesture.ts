import { gesture } from './tokens';

const msPerSecond = 1000;

/** Where a released item would come to rest: position + (velocity ÷ 1000) × d ÷ (1 − d) (MOT-032). */
export function project(
  position: number,
  velocityPxPerS: number,
  deceleration: number = gesture.projectionDeceleration,
): number {
  return position + ((velocityPxPerS / msPerSecond) * deceleration) / (1 - deceleration);
}

/** The distance shown for `overshoot` past a limit: (overshoot × size × c) ÷ (size + c × |overshoot|) (MOT-033). */
export function rubberband(overshoot: number, size: number, constant: number = gesture.rubberbandConstant): number {
  return (overshoot * size * constant) / (size + constant * Math.abs(overshoot));
}

/** The snap point nearest the projected position, not the release position (MOT-032). */
export function nearestSnap(position: number, velocityPxPerS: number, snaps: readonly [number, ...number[]]): number {
  const resting = project(position, velocityPxPerS);
  return snaps.reduce((best, snap) => (Math.abs(snap - resting) < Math.abs(best - resting) ? snap : best));
}
