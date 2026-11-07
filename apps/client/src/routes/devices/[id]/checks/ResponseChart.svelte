<script lang="ts">
  import { untrack } from 'svelte';
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
  /** The step of the newest beat; it counts the beats that came in since the chart was made (Chart update role). */
  let newest = $state(0);
  let seen: readonly ChartBeat[] = [];
  // Before the DOM updates, so the chart draws the new beats and the moved window together.
  $effect.pre(() => {
    const now = beats;
    newest = untrack(() => stepsAfter(seen, now, newest));
    seen = now;
  });
  const drawn = $derived(responseSteps(beats, newest));
  const series = $derived([{ label: 'Response time', colour: 1 as const, points: drawn.points }]);
  const title = $derived(`Response time of the last ${String(Math.min(beats.length, RESPONSE_POINTS))} heartbeats`);
</script>

<LineChart
  {title}
  {series}
  span={drawn.span}
  axis="open"
  format={msText}
  timeAxis={false}
  empty="No response times yet"
  {preference}
/>
