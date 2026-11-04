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
  import { createSessions, provideSessions, type SessionPort } from '$lib/sessions/sessions.svelte';
  import SessionPanes from '$lib/sessions/strip/SessionPanes.svelte';
  import SessionTabs from '$lib/sessions/strip/SessionTabs.svelte';

  const { children }: { children: Snippet } = $props();

  // WP-16 supplies the key-store token and the stored hub address here (handoff H-2).
  const hub = createHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve(null) });
  provideHub(hub);
  const clock = createClock(DATA_AGE_REFRESH_MS);

  // TEMPORARY until WP-14 T1 merges: replace with `sessionsApi(hub)` from `$lib/sessions/api`.
  // Until then no session can start; ending one that never started cannot happen.
  const notWired = () => Promise.reject(new Error('Sessions are not wired to the hub yet (WP-14 T1)'));
  const sessionsPort: SessionPort = { start: notWired, end: notWired };
  // WP-16 passes the `popOut` port as the second argument (WP-14 H-2); without it none is offered.
  const sessions = createSessions(sessionsPort);
  provideSessions(sessions);

  $effect(() => () => {
    // close() rejects only when a pending connect failed; the status already shows that.
    hub.close().catch((reason: unknown) => {
      // Fixed text and the error name only: the message is decided by the transport (LOG-007).
      console.warn('Hub connection attempt failed while closing:', reason instanceof Error ? reason.name : 'unknown');
    });
  });
</script>

<div class="shell">
  <Nav pathname={page.url.pathname} now={clock.current} onnavigate={() => sessions.deselect()} />
  <main>
    <SessionTabs />
    <div class="stage">
      <!-- The pages stay mounted under a selected session tab (WP-14 E-2, R-044). -->
      <div class="pages" class:covered={sessions.active !== null} inert={sessions.active !== null}>
        {#key page.url.pathname}
          <div class="page" transition:crossFade>
            {@render children()}
          </div>
        {/key}
      </div>
      <SessionPanes />
    </div>
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
    display: grid;
    grid-template-rows: auto 1fr;
    padding: var(--space-6);
  }

  .stage,
  .pages {
    display: grid;
  }

  .pages,
  .page {
    grid-area: 1 / 1;
  }

  .covered {
    visibility: hidden;
  }
</style>
