<script lang="ts">
  import ChartFrame from './ChartFrame.svelte';
  import type { SeriesColour } from './plot';
  import { valueScale, type TimeWindow, type ValueAxis } from './scales';

  interface Props {
    /** The chart's window, so the time labels are already the ones the chart will show. */
    span: TimeWindow;
    axis: ValueAxis;
    /** The value text of a percent chart; an open axis has no known values yet, so its labels stay empty. */
    format?: (value: number) => string;
    threshold?: number;
    timeAxis?: boolean;
    /** The chart's series names and colours, so a legend takes its place as it will with data. */
    legend?: readonly { readonly label: string; readonly colour: SeriesColour }[];
  }

  const { span, axis, format, threshold, timeAxis = true, legend = [] }: Props = $props();
  const scale = $derived(valueScale(axis, 0));
</script>

<!--
  Loading (5.7.3, D-3): the chart's box, grid and axes with no line, and no shimmer. Show it through `skeletonDelay`
  and swap it for the `LineChart` with `transition:crossFade`, both in one grid cell, so nothing moves (MOT-007).
-->
<ChartFrame {span} {scale} format={axis === 'percent' ? format : undefined} {threshold} {timeAxis} {legend} />
