<script lang="ts">
  import { motionPreference } from '$lib/motion/preference';
  import { doorDots } from './doors';

  const DOTS = doorDots(2600);
  /** Nearest frame to farthest: the frost blues, deep to light (the mark's own colours, D13). */
  const SHADES = ['--graphite-frost-10', '--graphite-frost-9', '--graphite-frost-8', '--graphite-frost-7', '--graphite-frost-7'];
  /** How far a point wanders, in mark units, and how long one wander takes, in milliseconds. */
  const DRIFT = 0.004;
  const DRIFT_PERIOD = 1800;

  const motion = motionPreference();
  let canvas: HTMLCanvasElement | undefined = $state();

  // The doors drift slowly; with reduced motion they are drawn once and stay still (MOT-040, D13).
  $effect(() => {
    const context = canvas?.getContext('2d');
    if (canvas === undefined || context === null || context === undefined) return;
    const still = motion.reduced;
    const style = getComputedStyle(canvas);
    const colours = SHADES.map((name) => style.getPropertyValue(name).trim());
    let frame = 0;

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
      const scale = Math.min(width, height) * 0.42;
      const size = ratio * 1.4;
      for (const dot of DOTS) {
        const drift = still ? 0 : Math.sin(time / DRIFT_PERIOD + dot.phase) * DRIFT;
        context.globalAlpha = 0.3 + dot.depth * 0.05;
        context.fillStyle = colours[Math.min(4, Math.floor(dot.depth))] ?? '';
        context.fillRect(width / 2 + (dot.x + drift) * scale, height / 2 + (dot.y + drift) * scale, size, size);
      }
      if (!still) frame = requestAnimationFrame(draw);
    }

    frame = requestAnimationFrame(draw);
    const resized = new ResizeObserver(() => {
      if (still) draw(0);
    });
    resized.observe(canvas);
    return () => {
      cancelAnimationFrame(frame);
      resized.disconnect();
    };
  });
</script>

<!-- The doors mark in points behind the sign-in form (UR-7, D13); decoration only, so hidden from assistive tech. -->
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
