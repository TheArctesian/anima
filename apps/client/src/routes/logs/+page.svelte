<script lang="ts">
  import { untrack } from 'svelte';
  import { LogFeed } from '$lib/events/panel/feed.svelte';
  import LogFilters from '$lib/events/panel/LogFilters.svelte';
  import LogList from '$lib/events/panel/LogList.svelte';
  import { useHub } from '$lib/hub/context';

  // The logs as a page (MOB-013): the phone layout has no logs panel, so the bar opens this page.
  const now = () => new Date();
  const feed = new LogFeed(useHub(), now);
  const rows = $derived(feed.log?.rows ?? []);

  $effect(() => {
    feed.follow(true);
  });

  /** Only the page coming and going starts and stops the feed; a filter change goes through `setFilter`. */
  $effect(() => {
    untrack(() => {
      feed.start();
    });
    return () => {
      feed.stop();
    };
  });
</script>

<svelte:head><title>Logs – Anima.ing</title></svelte:head>

<h1>Logs</h1>
<LogFilters
  filter={feed.filter}
  devices={[]}
  shown={feed.log === null ? null : rows.length}
  onChange={(next) => {
    feed.setFilter(next);
  }}
/>
<LogList {feed} {now} />

<style>
  h1 {
    margin: 0 0 var(--space-4);
    font-size: var(--text-2xl);
    font-weight: var(--weight-light);
    line-height: var(--leading-tight);
  }
</style>
