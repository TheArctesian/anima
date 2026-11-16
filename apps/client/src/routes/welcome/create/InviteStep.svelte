<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { loadClientAt } from '$lib/access/navigation';
  import { readStayAwake } from '$lib/desktop/stay-awake/shell';
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { RequestHub } from '$lib/hub/hub';
  import { linkApi } from '$lib/link/api';
  import InviteMaker from '$lib/link/InviteMaker.svelte';
  import { createClock } from '$lib/nav/clock.svelte';
  import { sleepNotice } from '$lib/stay-awake/stay-awake';

  interface Props {
    /** The hub this computer just signed in to. */
    hub: RequestHub;
    clock?: { readonly current: Date };
  }

  const { hub, clock = createClock(1000) }: Props = $props();
  // svelte-ignore state_referenced_locally
  const api = linkApi(hub);
  let notice = $state<string | null>(null);

  // This computer is the hub here (FRN-008): when Stay awake is off, its checks stop while it sleeps (AWK-005).
  onMount(() => {
    void readStayAwake().then((read) => {
      notice = read.ok ? sleepNotice(true, read.status) : null;
    });
  });
</script>

<p>The next device joins with an invite. Make one for each device; the hub lists them in Settings.</p>
<InviteMaker {api} now={clock.current} />
{#if notice !== null}
  <!-- The link loads a new page: the client starts again with the hub address just saved. -->
  <p class="notice" role="status">{notice} <a data-sveltekit-reload href={resolve('/settings')}>Change Stay awake in Settings</a>.</p>
{/if}
<FormButton tone="primary" onclick={() => loadClientAt('/devices')}>Done</FormButton>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .notice {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-warning);
  }

  a {
    color: var(--accent);
  }
</style>
