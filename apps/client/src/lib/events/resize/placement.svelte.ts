import { animate } from '$lib/motion/animate';
import { springMotion, type Motion } from '$lib/motion/options';
import type { MotionPreference } from '$lib/motion/preference';
import { COLLAPSED, draggedHeight, keyStop, releasedStop, THIRD, type Snaps, type Stop } from './resize';

interface Surroundings {
  readonly snaps: () => Snaps;
  /** The panel element; it is as tall as the 66 % stop and slides by `translate`, so nothing reflows (MOT-006). */
  readonly panel: () => HTMLElement | undefined;
  readonly preference: MotionPreference;
}

const instant: Motion = { type: 'instant' };

/** How far down the panel is drawn now (`translate`, in px), or null when nothing has moved it yet. */
function drawnOffset(panel: HTMLElement): number | null {
  const y = Number.parseFloat(getComputedStyle(panel).translate.split(' ')[1] ?? '');
  return Number.isNaN(y) ? null : y;
}

/**
 * How far the logs panel is open: the stop it rests at, the height it shows now and the motion between stops.
 * A drag moves it 1:1 with rubber-band at the limits; a release, a key or the disclosure button springs it to a stop.
 */
export class PanelPlacement {
  stop = $state<Stop>(THIRD);
  height = $state(0);
  #lastOpen: Stop = THIRD;
  #grabbedAt = 0;
  readonly #around: Surroundings;

  constructor(around: Surroundings) {
    this.#around = around;
  }

  /** Whether any of the content shows. */
  get open(): boolean {
    return this.height > this.#around.snaps()[COLLAPSED];
  }

  /** Puts the panel at its stop at once: first paint, and a page that changed size. */
  settle(): void {
    this.height = this.#around.snaps()[this.stop];
    this.#place(instant);
  }

  /** Takes hold of the panel where it is drawn now, so a grab in the middle of a spring does not jump (MOT-002). */
  grab(): void {
    const panel = this.#around.panel();
    const drawn = panel === undefined ? null : drawnOffset(panel);
    this.height = drawn === null ? this.height : this.#around.snaps()[2] - drawn;
    this.#grabbedAt = this.height;
  }

  /** The top edge is `delta` px below where it was grabbed; the panel follows 1:1 (MOT-030). */
  move(delta: number): void {
    this.height = draggedHeight(this.#grabbedAt - delta, this.#around.snaps());
    this.#place(instant);
  }

  /** `velocity` is the top edge's speed upward in px/s; the spring starts with it (MOT-031). */
  release(velocity: number): void {
    const stop = releasedStop(this.height, velocity, this.#around.snaps());
    this.#goTo(stop, springMotion('momentum', this.#around.preference.reduced, -velocity));
  }

  /** Moves by a key; returns whether the key was one the separator uses. */
  press(key: string): boolean {
    const stop = keyStop(key, this.stop);
    if (stop === null) return false;
    this.#goTo(stop, springMotion('default', this.#around.preference.reduced));
    return true;
  }

  /** The disclosure: closes an open panel, or opens a closed one to where it was. */
  toggle(): void {
    this.#goTo(this.stop === COLLAPSED ? this.#lastOpen : COLLAPSED, springMotion('default', this.#around.preference.reduced));
  }

  #goTo(stop: Stop, motion: Motion): void {
    this.stop = stop;
    if (stop !== COLLAPSED) this.#lastOpen = stop;
    this.height = this.#around.snaps()[stop];
    this.#place(motion);
  }

  #place(motion: Motion): void {
    const panel = this.#around.panel();
    if (panel === undefined) return;
    const { snaps } = this.#around;
    animate(panel, { translate: { y: snaps()[2] - this.height } }, motion);
  }
}
