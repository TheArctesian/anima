<script lang="ts">
  import { untrack } from 'svelte';
  import { LogFeed } from '$lib/events/panel/feed.svelte';
  import LogFilters from '$lib/events/panel/LogFilters.svelte';
  import LogList from '$lib/events/panel/LogList.svelte';
  import { useHub } from '$lib/hub/context';
  import { phoneLayout } from '$lib/nav/phone/layout.svelte';
  import LogChips from './phone/LogChips.svelte';
  import LogsHead from './phone/LogsHead.svelte';
  import './phone/logs.css';

  // The logs as a page (MOB-013): the phone layout has no logs panel, so the bar opens this page.
  const now = () => new Date();
  const feed = new LogFeed(useHub(), now);
  const rows = $derived(feed.log?.rows ?? []);
  const phone = $derived(phoneLayout());

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

<div data-logs-page={phone ? '' : undefined}>
  {#if phone}
    <LogsHead text={feed.filter.text} onTextChange={(text) => { feed.setFilter({ ...feed.filter, text }); }} />
    <LogChips filter={feed.filter} onChange={(next) => { feed.setFilter(next); }} />
  {:else}
    <div class="pagehead"><h1>Logs</h1></div>
    <LogFilters
      filter={feed.filter}
      devices={[]}
      shown={feed.log === null ? null : rows.length}
      onChange={(next) => {
        feed.setFilter(next);
      }}
    />
  {/if}
  <LogList {feed} {now} />
</div>

<style>
  h1 {
    margin: 0 0 var(--space-4);
    font-size: var(--text-2xl);
    font-weight: var(--weight-light);
    line-height: var(--leading-tight);
  }
</style>
