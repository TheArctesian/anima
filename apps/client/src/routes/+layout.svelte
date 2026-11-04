<script lang="ts">
  import { page } from '$app/state';
  import { untrack, type Snippet } from 'svelte';
  import '../styles/global.css';
  import { provideHub } from '$lib/hub/context';
  import { DEFAULT_HUB_ADDRESS, createHub } from '$lib/hub/hub';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { DATA_AGE_REFRESH_MS } from '$lib/nav/nav';
  import { createClock } from '$lib/nav/clock.svelte';
  import Nav from '$lib/nav/Nav.svelte';
  import { sessionsApi } from '$lib/sessions/api';
  import { createSessions, provideSessions, type SessionPort } from '$lib/sessions/sessions.svelte';
  import SessionPanes from '$lib/sessions/strip/SessionPanes.svelte';
  import SessionTabs from '$lib/sessions/strip/SessionTabs.svelte';
  import { sessionStart } from '$lib/sessions/view/start';

  const { children }: { children: Snippet } = $props();

  // WP-16 supplies the key-store token and the stored hub address here (handoff H-2).
  const hub = createHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve(null) });
  provideHub(hub);
  const clock = createClock(DATA_AGE_REFRESH_MS);

  // Each session view starts its own sessions, since only it knows its display (D-7); the store ends them (D-11).
  const api = sessionsApi(hub);
  const sessionsPort: SessionPort = {
    start: (deviceId) => sessionStart(api, deviceId, hub.address.origin),
    end: (sessionId) => api.endSession(sessionId),
  };
  // WP-16 passes the `popOut` port as the second argument (WP-14 H-2); without it none is offered.
  const sessions = createSessions(sessionsPort);
  provideSessions(sessions);
  // A route change (link, back, goto) shows the pages; the session tabs stay open (D-11).
  $effect(() => {
    void page.url.pathname;
    untrack(() => {
      sessions.deselect();
    });
  });

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
      <SessionPanes {api} />
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

  /* At least the window's height, so a selected session's stage fills what the strip leaves (D-7). */
  main {
    display: grid;
    align-self: stretch;
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

  /* Covered pages stay mounted but give the stage no height, so a session pane is sized by the window (D-7). */
  .covered {
    visibility: hidden;
    block-size: 0;
    overflow: hidden;
  }
</style>
