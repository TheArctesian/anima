<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { parseListQuery, writeListQuery, type ListQuery } from '$lib/devices/query';
  import DeviceScreen from '$lib/devices/screen/DeviceScreen.svelte';

  const query = $derived(parseListQuery(page.url.searchParams));

  /** The query goes to the URL, so Back and Forward restore the list (D-4). */
  function changeQuery(next: ListQuery): void {
    const search = writeListQuery(next).toString();
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the query string is added
    void goto(`${resolve('/devices')}${search === '' ? '' : `?${search}`}`, {
      replaceState: true,
      keepFocus: true,
      noScroll: true,
    });
  }

  /** WP-13 replaces this with its Add device form (H-1). */
  function addDevice(): void {
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); /devices/new is WP-13's route
    void goto(`${resolve('/devices')}/new`);
  }

  /** WP-14 supplies the session open (H-2). */
  function connect(): void {
    return undefined;
  }
</script>

<section>
  <h1>Devices</h1>
  <DeviceScreen {query} onQueryChange={changeQuery} onAddDevice={addDevice} onConnect={connect} />
</section>

<style>
  /* A fixed block size from tokens, so the font swap cannot move the page (WP-11 H-7). */
  h1 {
    box-sizing: border-box;
    block-size: calc(var(--text-2xl) * var(--leading-tight));
    margin: 0 0 var(--space-4);
    overflow: hidden;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    white-space: nowrap;
  }
</style>
