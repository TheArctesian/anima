import { animate as run, createGeneratorEasing, cubicBezier, motionValue, spring, type MotionValue } from 'motion';
import type { EaseName, Motion, SpringMotion } from './options';
import { ease } from './tokens';

export interface Axes {
  readonly x?: number;
  readonly y?: number;
}

/** The only properties Anima animates (MOT-006): `translate` in pixels, `scale`, `opacity`. */
export interface MotionTarget {
  readonly translate?: Axes;
  readonly scale?: number | Axes;
  readonly opacity?: number;
}

/** A started animation. `finished` settles when every value arrives, is stopped or is interrupted. */
export interface Playback {
  stop(): void;
  readonly finished: Promise<void>;
}

export type Animatable = Element & ElementCSSInlineStyle;

type Channel = 'x' | 'y' | 'scaleX' | 'scaleY' | 'opacity';

interface Track {
  readonly value: MotionValue<number>;
  /** Writes this value, with its partner axis, to the element's style. */
  readonly draw: () => void;
  running?: { stop(): void };
}

type Tracks = Record<Channel, Track>;

const msPerSecond = 1000;
const tracksOf = new WeakMap<Animatable, Tracks>();

function axisPair(text: string, single: (x: number) => number, rest: number): [number, number] {
  if (text === '' || text === 'none') return [rest, rest];
  const [x = rest, y] = text.trim().split(/\s+/).map((part) => Number.parseFloat(part));
  return [x, y ?? single(x)];
}

/** The values on screen now, so a new motion starts where the old one is (MOT-002). */
function rendered(element: Animatable): Record<Channel, number> {
  const style = getComputedStyle(element);
  const [x, y] = axisPair(style.translate, () => 0, 0);
  const [scaleX, scaleY] = axisPair(style.scale, (sx) => sx, 1);
  const opacity = Number.parseFloat(style.opacity);
  return { x, y, scaleX, scaleY, opacity: Number.isNaN(opacity) ? 1 : opacity };
}

function tracksFor(element: Animatable): Tracks {
  const known = tracksOf.get(element);
  if (known) return known;
  const now = rendered(element);
  const x = motionValue(now.x);
  const y = motionValue(now.y);
  const scaleX = motionValue(now.scaleX);
  const scaleY = motionValue(now.scaleY);
  const opacity = motionValue(now.opacity);
  const translate = () => {
    element.style.translate = `${String(x.get())}px ${String(y.get())}px`;
  };
  const scale = () => {
    element.style.scale = `${String(scaleX.get())} ${String(scaleY.get())}`;
  };
  const fade = () => {
    element.style.opacity = String(opacity.get());
  };
  const tracks: Tracks = {
    x: { value: x, draw: translate },
    y: { value: y, draw: translate },
    scaleX: { value: scaleX, draw: scale },
    scaleY: { value: scaleY, draw: scale },
    opacity: { value: opacity, draw: fade },
  };
  for (const track of Object.values(tracks)) track.value.on('change', track.draw);
  tracksOf.set(element, tracks);
  return tracks;
}

function goals(target: MotionTarget): [Channel, number][] {
  const scale = typeof target.scale === 'number' ? { x: target.scale, y: target.scale } : target.scale;
  const pairs: [Channel, number | undefined][] = [
    ['x', target.translate?.x],
    ['y', target.translate?.y],
    ['scaleX', scale?.x],
    ['scaleY', scale?.y],
    ['opacity', target.opacity],
  ];
  return pairs.filter((pair): pair is [Channel, number] => pair[1] !== undefined);
}

const partner: Partial<Record<Channel, Channel>> = { x: 'y', y: 'x', scaleX: 'scaleY', scaleY: 'scaleX' };

/** An axis that does not move is still written with its partner, so it must hold the value on screen. */
function holdPartners(tracks: Tracks, moving: readonly Channel[], now: Record<Channel, number>): void {
  for (const channel of moving) {
    const other = partner[channel];
    if (other === undefined || moving.includes(other)) continue;
    const { value, running } = tracks[other];
    if (!running && value.get() !== now[other]) value.jump(now[other]);
  }
}

function engineOptions(motion: Exclude<Motion, { type: 'instant' }>, inherited: number) {
  if (motion.type === 'tween') return { duration: motion.durationMs / msPerSecond, ease: motion.ease };
  const { stiffness, damping, mass, velocity = inherited } = motion;
  return { type: 'spring', stiffness, damping, mass, velocity } as const;
}

const settled: Playback = { stop: () => undefined, finished: Promise.resolve() };

function start(track: Track, from: number, to: number, motion: Motion): Playback {
  const inherited = track.value.getVelocity();
  track.running?.stop();
  if (motion.type === 'instant') {
    track.value.jump(to);
    track.draw();
    return settled;
  }
  const controls = run(track.value, [from, to], engineOptions(motion, inherited));
  let resolve: () => void = () => undefined;
  const finished = new Promise<void>((done) => {
    resolve = done;
  });
  const settle = () => {
    if (track.running === running) delete track.running;
    resolve();
  };
  const running = {
    stop: () => {
      if (track.running === running) controls.stop();
      settle();
    },
  };
  track.running = running;
  controls.finished.then(settle, settle);
  return { stop: running.stop, finished };
}

/**
 * Animates `element` to `target` from the values on screen now; a later call on the same
 * values interrupts this one and keeps its velocity (MOT-002). A spring's `velocity` applies
 * to every value in `target`; animate the axes in separate calls when they differ (MOT-035).
 * Instant motion is applied before this returns, so no caller waits for a timer (MOT-042).
 */
export function animate(element: Animatable, target: MotionTarget, motion: Motion): Playback {
  const tracks = tracksFor(element);
  const now = rendered(element);
  const moving = goals(target);
  holdPartners(
    tracks,
    moving.map(([channel]) => channel),
    now,
  );
  return together(moving.map(([channel, to]) => start(tracks[channel], now[channel], to, motion)));
}

/** One playback for several: `stop` stops all, `finished` settles when all have settled. */
export function together(playbacks: readonly Playback[]): Playback {
  return {
    stop: () => {
      for (const playback of playbacks) playback.stop();
    },
    finished: Promise.all(playbacks.map((playback) => playback.finished)).then(() => undefined),
  };
}

/** A progress curve for a CSS transition: the ease token as a function of time 0 to 1. */
export function easeCurve(name: EaseName): (t: number) => number {
  const [x1, y1, x2, y2] = ease[name];
  return cubicBezier(x1, y1, x2, y2);
}

/** The spring sampled as a progress curve over its settling time, for CSS transitions. */
export function springCurve(motion: SpringMotion): { easing: (t: number) => number; durationMs: number } {
  const { stiffness, damping, mass } = motion;
  const sampled = createGeneratorEasing({ stiffness, damping, mass }, undefined, spring);
  return { easing: sampled.ease, durationMs: sampled.duration * msPerSecond };
}
