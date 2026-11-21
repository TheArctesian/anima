<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { agentUsageApi } from '$lib/agent-usage/api';
  import AgentUsageScreen from '$lib/agent-usage/AgentUsageScreen.svelte';
  import { parseUsageQuery, usageRequest, writeUsageQuery, type UsageQuery } from '$lib/agent-usage/query';
  import { UsageStore } from '$lib/agent-usage/store.svelte';
  import { daemonHere, daemonHereLine } from '$lib/desktop/daemon/state.svelte';
  import { hubIsHosted } from '$lib/hosted/mark';
  import { useHub } from '$lib/hub/context';
  import { LiveCharges } from '$lib/metrics/battery/charges.svelte';

  const hub = useHub();
  const api = agentUsageApi(hub);
  const hosted = hubIsHosted(hub.address.origin);
  /** On a hosted hub the page says where the daemon of this computer stands; elsewhere it says where to install one. */
  const hostedLine = $derived(hosted ? daemonHereLine(daemonHere.current) : undefined);
  const store = new UsageStore(api.read);
  /** The device rows show the charge of a battery from the batches of its daemon (MET-032). */
  const charges = new LiveCharges();
  const query = $derived(parseUsageQuery(page.url.searchParams));

  /** The choices are in the address, so Back, Forward and a copied link restore the page. */
  function changeQuery(next: UsageQuery): void {
    const search = writeUsageQuery(next).toString();
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the query string is added
    void goto(`${resolve('/agents/usage')}${search === '' ? '' : `?${search}`}`, {
      replaceState: true,
      keepFocus: true,
      noScroll: true,
    });
  }

  $effect(() => {
    charges.start(hub);
    return () => {
      charges.stop();
    };
  });

  /** Each change of the address reads the use again; the days end today by the clock of this computer. */
  $effect(() => {
    void store.load(usageRequest(query, new Date()));
  });
</script>

<AgentUsageScreen {store} {query} {hostedLine} {charges} onQueryChange={changeQuery} onRetry={() => void store.load(usageRequest(query, new Date()))} />
