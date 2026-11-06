<script lang="ts">
  import { animate } from '$lib/motion/animate';
  import { springMotion } from '$lib/motion/options';
  import { motionPreference, type MotionPreference } from '$lib/motion/preference';
  import { CHART_BOX, CHART_POINTS, CHART_STEP, chartLines, gainedNewest, type ChartBeat } from './chart';
  import { msText } from './rows';

  interface Props {
    /** Newest first, as the history lists them. */
    beats: readonly ChartBeat[];
    preference?: MotionPreference;
  }

  const { beats, preference = motionPreference() }: Props = $props();
  const lines = $derived(chartLines(beats));
  const label = $derived(
    beats.length === 0
      ? 'No response times yet'
      : `Response time of the last ${String(Math.min(beats.length, CHART_POINTS))} heartbeats, latest ${msText(beats[0]?.responseTimeMs)}, top of the chart ${msText(lines.maxMs)}`,
  );
  let svg = $state<SVGSVGElement>();
  let line = $state<SVGGElement>();
  let before: readonly ChartBeat[] | undefined;

  /**
   * Chart update (5.7.3): the new point is drawn at the right edge, so the line starts one step to the right and
   * moves left on `spring-default`, from where it is on screen (MOT-002). Instant under reduced motion.
   */
  $effect(() => {
    const now = beats;
    const was = before;
    before = now;
    if (svg === undefined || line === undefined || was === undefined || !gainedNewest(was, now)) return;
    const pixels = (svg.getBoundingClientRect().width / CHART_BOX.width) * CHART_STEP;
    const shown = Number.parseFloat(getComputedStyle(line).translate.split(' ')[0] ?? '') || 0;
    animate(line, { translate: { x: shown + pixels, y: 0 } }, { type: 'instant' });
    animate(line, { translate: { x: 0, y: 0 } }, springMotion('default', preference.reduced));
  });
</script>

<svg
  bind:this={svg}
  class="chart"
  viewBox="0 0 {CHART_BOX.width} {CHART_BOX.height}"
  preserveAspectRatio="none"
  role="img"
  aria-label={label}
>
  <g bind:this={line}>
    {#each lines.segments as points, index (index)}
      <polyline {points} />
    {/each}
  </g>
</svg>

<style>
  /* A fixed block size from tokens: new data never moves the page (MOT-007). */
  .chart {
    display: block;
    inline-size: 100%;
    block-size: calc(var(--space-16) * 2);
    overflow: hidden;
    border-block-end: var(--border-thin) solid var(--border);
  }

  polyline {
    fill: none;
    stroke: var(--accent);
    stroke-width: var(--border-focus);
    vector-effect: non-scaling-stroke;
  }
</style>
