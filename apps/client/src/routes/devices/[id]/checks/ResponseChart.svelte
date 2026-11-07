<script lang="ts">
  import LineChart from '$lib/charts/LineChart.svelte';
  import type { MotionPreference } from '$lib/motion/preference';
  import { RESPONSE_POINTS, responseSteps, stepsAfter, type ChartBeat } from './response';
  import { msText } from './rows';

  interface Props {
    /** Newest first, as the history lists them. */
    beats: readonly ChartBeat[];
    preference?: MotionPreference;
  }

  const { beats, preference }: Props = $props();
  // The step count remembers the beats it last saw, so it is kept outside reactive state.
  let seen: readonly ChartBeat[] = [];
  let steps = 0;
  const newest = $derived.by(() => {
    steps = stepsAfter(seen, beats, steps);
    seen = beats;
    return steps;
  });
  const drawn = $derived(responseSteps(beats, newest));
  const series = $derived([{ label: 'Response time', colour: 1 as const, points: drawn.points }]);
  const title = $derived(`Response time of the last ${String(Math.min(beats.length, RESPONSE_POINTS))} heartbeats`);
</script>

<LineChart
  {title}
  {series}
  window={drawn.window}
  axis="open"
  format={msText}
  timeAxis={false}
  empty="No response times yet"
  {preference}
/>
