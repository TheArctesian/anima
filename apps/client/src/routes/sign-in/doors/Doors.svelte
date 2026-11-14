<script lang="ts">
  import { motionPreference } from '$lib/motion/preference';
  import { doorDots, inDoorway, OPENING, type DoorDot } from './doors';

  interface Props {
    /** The element that stands in the doorway: the mark is scaled and placed so its opening frames it (D13). */
    doorway?: HTMLElement | undefined;
  }

  const { doorway }: Props = $props();
  const DOTS = doorDots(3200);
  /** The points outside the doorway, drawn when the form stands in it. */
  const AROUND = DOTS.filter((dot) => !inDoorway(dot));
  /** Room between the form and the door's jambs, as a share of the form's width; tall enough for step 1. */
  const ROOM = 1.2;
  /** Nearest frame to farthest: the frost blues, deep to light (the mark's own colours, D13). */
  const SHADES = ['--graphite-frost-10', '--graphite-frost-9', '--graphite-frost-8', '--graphite-frost-7', '--graphite-frost-7'];
  /** On a light ground the pale blues vanish, so the light theme draws the deep ones, a little stronger. */
  const SHADES_LIGHT = ['--graphite-frost-10', '--graphite-accent-deep', '--graphite-frost-10', '--graphite-accent-deep', '--graphite-frost-9'];
  /** How far a point wanders, in mark units, and how long one wander takes, in milliseconds. */
  const DRIFT = 0.004;
  const DRIFT_PERIOD = 1800;

  interface Placement {
    readonly scale: number;
    readonly x: number;
    readonly y: number;
    readonly dots: readonly DoorDot[];
  }

  const motion = motionPreference();
  let canvas: HTMLCanvasElement | undefined = $state();

  // The doors drift slowly; with reduced motion they are drawn once and stay still (MOT-040, D13).
  $effect(() => {
    const context = canvas?.getContext('2d');
    if (canvas === undefined || context === null || context === undefined) return;
    const still = motion.reduced;
    const style = getComputedStyle(canvas);
    const light = style.colorScheme === 'light';
    const colours = (light ? SHADES_LIGHT : SHADES).map((name) => style.getPropertyValue(name).trim());
    const strength = light ? 0.45 : 0.3;
    let frame = 0;

    /**
     * The mark's scale and origin in canvas pixels, and the points to draw. Around the doorway element when the
     * doorway's jambs fit on screen beside it; on a narrow window, the whole mark in the room above it.
     */
    function placement(target: HTMLCanvasElement, ratio: number, width: number, height: number): Placement {
      if (doorway === undefined) return { scale: Math.min(width, height) * 0.42, x: width / 2, y: height / 2, dots: DOTS };
      const box = doorway.getBoundingClientRect();
      const ground = target.getBoundingClientRect();
      // From the width alone, so the doors keep their size as the form changes step (D13).
      const scale = (box.width / (OPENING.right - OPENING.left)) * ROOM * ratio;
      const middleX = (box.left - ground.left + box.width / 2) * ratio;
      const middleY = (box.top - ground.top + box.height / 2) * ratio;
      const left = middleX - ((OPENING.left + OPENING.right) / 2) * scale;
      // The outer frame runs from -1 to 1; the doorway needs at least the second frame's jambs on screen.
      if (left - 0.6 * scale >= 0 && left + 0.75 * scale <= width) {
        return { scale, x: left, y: middleY - ((OPENING.top + OPENING.bottom) / 2) * scale, dots: AROUND };
      }
      const room = (box.top - ground.top) * ratio;
      const above = Math.min(width * 0.8, room * 0.8) / 2;
      return { scale: above, x: width / 2, y: room / 2, dots: DOTS };
    }

    function draw(time: number): void {
      if (canvas === undefined || context === null || context === undefined) return;
      const ratio = Math.min(devicePixelRatio, 2);
      const width = canvas.clientWidth * ratio;
      const height = canvas.clientHeight * ratio;
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      context.clearRect(0, 0, width, height);
      const { scale, x, y, dots } = placement(canvas, ratio, width, height);
      const size = ratio * 1.4;
      for (const dot of dots) {
        const drift = still ? 0 : Math.sin(time / DRIFT_PERIOD + dot.phase) * DRIFT;
        context.globalAlpha = strength + dot.depth * 0.05;
        context.fillStyle = colours[Math.min(4, Math.floor(dot.depth))] ?? '';
        context.fillRect(x + (dot.x + drift) * scale, y + (dot.y + drift) * scale, size, size);
      }
      if (!still) frame = requestAnimationFrame(draw);
    }

    frame = requestAnimationFrame(draw);
    const resized = new ResizeObserver(() => {
      if (still) draw(0);
    });
    resized.observe(canvas);
    if (doorway !== undefined) resized.observe(doorway);
    return () => {
      cancelAnimationFrame(frame);
      resized.disconnect();
    };
  });
</script>

<!-- The doors mark in points, with the sign-in form standing in its doorway (UR-7, D13); decoration only, so hidden from
     assistive tech. -->
<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
  canvas {
    position: absolute;
    inset: 0;
    inline-size: 100%;
    block-size: 100%;
    pointer-events: none;
  }
</style>
