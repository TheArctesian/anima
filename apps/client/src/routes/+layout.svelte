<script lang="ts">
  import { page } from '$app/state';
  import type { Snippet } from 'svelte';
  import '../styles/global.css';
  import { provideHub } from '$lib/hub/context';
  import { DEFAULT_HUB_ADDRESS, createHub } from '$lib/hub/hub';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { createClock } from '$lib/nav/clock.svelte';
  import Nav from '$lib/nav/Nav.svelte';

  const { children }: { children: Snippet } = $props();

  // WP-16 supplies the key-store token and the stored hub address here (handoff H-2).
  const hub = createHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve(null) });
  provideHub(hub);
  const clock = createClock(30_000);

  $effect(() => () => {
    // close() rejects only when a pending connect failed; the status already shows that.
    hub.close().catch((reason: unknown) => {
      console.warn('Hub connection attempt failed while closing:', reason instanceof Error ? reason.message : reason);
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
  .shell {
    display: grid;
    grid-template-columns: auto 1fr;
    min-height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-ui);
  }

  main {
    display: grid;
    padding: var(--space-6);
  }

  .page {
    grid-area: 1 / 1;
  }
</style>
