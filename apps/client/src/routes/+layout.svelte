<script lang="ts">
  import { page } from '$app/state';
  import type { Snippet } from 'svelte';
  import '../styles/global.css';
  import { provideHub } from '$lib/hub/context';
  import { DEFAULT_HUB_ADDRESS, createHub } from '$lib/hub/hub';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { DATA_AGE_REFRESH_MS } from '$lib/nav/nav';
  import { createClock } from '$lib/nav/clock.svelte';
  import Nav from '$lib/nav/Nav.svelte';

  const { children }: { children: Snippet } = $props();

  // WP-16 supplies the key-store token and the stored hub address here (handoff H-2).
  const hub = createHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve(null) });
  provideHub(hub);
  const clock = createClock(DATA_AGE_REFRESH_MS);

  $effect(() => () => {
    // close() rejects only when a pending connect failed; the status already shows that.
    hub.close().catch((reason: unknown) => {
      // Fixed text and the error name only: the message is decided by the transport (LOG-007).
      console.warn('Hub connection attempt failed while closing:', reason instanceof Error ? reason.name : 'unknown');
    });
  });
</script>

<div class="shell">
  <Nav pathname={page.url.pathname} now={clock.current} />
  <main>
    {#key page.url.pathname}
      <div class="page" transition:crossFade>
        {@render children()}
      </div>
    {/key}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
  }

  .shell {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: start;
    min-height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-ui);
  }

  main {
    min-inline-size: 0;
    display: grid;
    padding: var(--space-6);
  }

  .page {
    grid-area: 1 / 1;
    min-inline-size: 0;
  }
</style>
