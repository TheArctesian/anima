<script lang="ts">
  import { page } from '$app/state';
  import { tick, untrack, type Snippet } from 'svelte';
  import '../styles/global.css';
  import { guardStart, holdsAt, reloadClient, startAt } from '$lib/access/navigation';
  import { provideHub } from '$lib/hub/context';
  import { readAccessToken } from '$lib/desktop/token';
  import { createHub, type HubAddress } from '$lib/hub/hub';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { DATA_AGE_REFRESH_MS } from '$lib/nav/nav';
  import { watchIconOnly } from '$lib/nav/icon-only';
  import { createClock } from '$lib/nav/clock.svelte';
  import Nav from '$lib/nav/Nav.svelte';
  import { sessionsApi } from '$lib/sessions/api';
  import { restoredScroll, type CoveredScroll } from '$lib/sessions/covered';
  import { createSessions, provideSessions, type SessionPort } from '$lib/sessions/sessions.svelte';
  import SessionPanes from '$lib/sessions/strip/SessionPanes.svelte';
  import SessionTabs from '$lib/sessions/strip/SessionTabs.svelte';
  import { sessionStart } from '$lib/sessions/view/start';
  import { applyTypefaces, storedTypefaces } from '$lib/typeface/typeface';
  import { provideDesktopUpdates } from '$lib/updates/desktop.svelte';
  import UpdateBanner from '$lib/updates/UpdateBanner.svelte';

  const { children }: { children: Snippet } = $props();

  // The operator's fonts apply before the first render, so the text does not change family after it shows (UI-020).
  applyTypefaces(document.documentElement, storedTypefaces(localStorage));
  // The bar shows icons only below the compact breakpoint; set before the first render so the width never changes after load.
  let iconOnly = $state(false);
  const stopWatchingWidth = watchIconOnly(document.documentElement, window, (now) => {
    iconOnly = now;
  });
  $effect(() => stopWatchingWidth);

  // Decided once, from the page the browser loaded: a bare route (the share viewer) renders only its page, with no
  // operator hub, socket, guard or shell (SEC-002, D-7, E-3); a navigation across that line loads the whole client.
  const start = startAt(page.url);
  // The router can show a page across that line without asking first (a navigation started while another is in
  // flight, a load redirect): such a page never renders, and the whole client starts afresh at its address.
  const holds = $derived(holdsAt(start, page.url));
  $effect(() => {
    if (!holds) reloadClient();
  });
  const shell = start.kind === 'shell' ? startShell(start.address) : null;

  /** The operator shell's parts: the hub and its start guard, the clock, updates and the session strip. */
  function startShell(address: HubAddress) {
    // The hub address is the stored setting on desktop and the serving origin in a browser (CLI-002, E-2); the token is
    // read from the desktop key store per call, and is none in a browser, where the cookie travels by itself (CLI-003).
    const hub = createHub({ address, token: readAccessToken });
    provideHub(hub);
    guardStart(page.url, hub);
    const clock = createClock(DATA_AGE_REFRESH_MS);
    provideDesktopUpdates();

    // Each session view starts its own sessions, since only it knows its display (D-7); the store ends them (D-11).
    const api = sessionsApi(hub);
    const sessionsPort: SessionPort = {
      start: (deviceId) => sessionStart(api, deviceId, hub.address.origin),
      end: (sessionId) => api.endSession(sessionId),
    };
    // WP-16 passes its pop-out window port as the second argument when `capabilities().popOut` is yes (WP-42 D-4,
    // WP-14 H-2); no platform says yes yet, so none is offered.
    const sessions = createSessions(sessionsPort);
    provideSessions(sessions);
    // A route change (link, back, goto) shows the pages; the session tabs stay open (D-11).
    $effect(() => {
      void page.url.pathname;
      untrack(() => {
        sessions.deselect();
      });
    });

    // A covered page has no height, so the document loses its scroll position: read it before the session covers the
    // page and give it back when the same page shows again (WP-14 E-2). A nav click is a navigation, so it drops the
    // saved position and keeps the scroll the router gives it (see `navigate`).
    let covered: CoveredScroll | null = null;
    $effect.pre(() => {
      const covering = sessions.active !== null;
      untrack(() => {
        if (covering && covered === null) {
          covered = { path: page.url.pathname, y: window.scrollY };
        } else if (!covering && covered !== null) {
          const saved = covered;
          covered = null;
          void tick().then(() => {
            const y = restoredScroll(saved, page.url.pathname);
            if (y !== undefined) window.scrollTo(0, y);
          });
        }
      });
    });

    function navigate() {
      covered = null;
      sessions.deselect();
    }

    $effect(() => () => {
      // close() rejects only when a pending connect failed; the status already shows that.
      hub.close().catch((reason: unknown) => {
        // Fixed text and the error name only: the message is decided by the transport (LOG-007).
        console.warn('Hub connection attempt failed while closing:', reason instanceof Error ? reason.name : 'unknown');
      });
    });

    return { api, clock, sessions, navigate };
  }
</script>

{#if shell !== null}
  {@const { api, clock, sessions, navigate } = shell}
  <div class="shell">
    <Nav pathname={page.url.pathname} now={clock.current} onnavigate={navigate} {iconOnly} />
    <main data-shifts-with-banner>
      <SessionTabs />
      <div class="stage">
        <!-- The pages stay mounted under a selected session tab (WP-14 E-2, R-044). -->
        <div class="pages" class:covered={sessions.active !== null} inert={sessions.active !== null}>
          {#key page.url.pathname}
            <div class="page" transition:crossFade>
              {#if holds}{@render children()}{/if}
            </div>
          {/key}
        </div>
        <SessionPanes {api} />
      </div>
    </main>
    <UpdateBanner />
  </div>
{:else if start.kind === 'refused'}
  <p class="refused" role="alert">{start.message}</p>
{:else if holds}
  {@render children()}
{/if}

<style>
  :global(body) {
    margin: 0;
  }

  .refused {
    margin: 0;
    padding: var(--space-6);
    min-height: 100vh;
    background: var(--bg);
    color: var(--text);
  }

  .shell {
    display: grid;
    /* The page column may shrink below its content's width, so a narrow window never scrolls sideways (UI-009). */
    grid-template-columns: auto minmax(0, 1fr);
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
    grid-template-columns: minmax(0, 1fr);
    min-inline-size: 0;
    padding: var(--space-6);
  }

  /* Below the compact breakpoint the unreachable strip takes the top of the window; the page starts below it. */
  :global(:root[data-nav='icons'][data-hub-banner]) main {
    padding-block-start: calc(var(--space-6) + var(--nav-banner-block-size));
  }

  /* One column that may shrink below its content, so neither the pages nor a session widen the document (UI-009). */
  .stage,
  .pages {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-inline-size: 0;
  }

  .pages,
  .page {
    grid-area: 1 / 1;
    min-inline-size: 0;
  }

  /* Covered pages stay mounted but give the stage no height, so a session pane is sized by the window (D-7). */
  .covered {
    visibility: hidden;
    block-size: 0;
    overflow: hidden;
  }
</style>
