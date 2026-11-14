<script lang="ts">
  import { page } from '$app/state';
  import { tick, untrack, type Snippet } from 'svelte';
  import '../styles/global.css';
  import { guardStart, showsSignedOutPage, startAt } from '$lib/access/navigation';
  import { provideDesktopBuilder } from '$lib/builds/desktop.svelte';
  import { provideHub } from '$lib/hub/context';
  import { readAccessToken } from '$lib/desktop/token';
  import { createHub, type HubAddress } from '$lib/hub/hub';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { shiftShell } from '$lib/nav/banner-shift';
  import { DATA_AGE_REFRESH_MS } from '$lib/nav/nav';
  import { storedCollapsed, storeCollapsed } from '$lib/nav/collapse/collapse';
  import { watchIconOnly } from '$lib/nav/collapse/icon-only';
  import { releaseWidth } from '$lib/nav/collapse/resize';
  import { fadeInWords } from '$lib/nav/collapse/words';
  import { createClock } from '$lib/nav/clock.svelte';
  import Nav from '$lib/nav/Nav.svelte';
  import { capabilities } from '$lib/platform/platform';
  import { sessionsApi } from '$lib/sessions/api';
  import { restoredScroll, type CoveredScroll } from '$lib/sessions/covered';
  import { createSessions, provideSessions, type SessionPort } from '$lib/sessions/sessions.svelte';
  import SessionPanes from '$lib/sessions/strip/SessionPanes.svelte';
  import SessionTabs from '$lib/sessions/strip/SessionTabs.svelte';
  import { sessionStart } from '$lib/sessions/view/start';
  import { applyPalette, storedPalette } from '$lib/palette/palette';
  import { applyMotion, storedMotion } from '$lib/theme/motion-choice';
  import { applyTheme, storedTheme } from '$lib/theme/theme';
  import { applyTypefaces, storedTypefaces } from '$lib/typeface/typeface';
  import { provideDesktopUpdates } from '$lib/updates/desktop.svelte';
  import UpdateBanner from '$lib/updates/UpdateBanner.svelte';

  const { children }: { children: Snippet } = $props();

  // The operator's fonts apply before the first render, so the text does not change family after it shows (UI-020).
  applyTypefaces(document.documentElement, storedTypefaces(localStorage));
  applyTheme(document.documentElement, storedTheme(localStorage));
  applyPalette(document.documentElement, storedPalette(localStorage));
  applyMotion(document.documentElement, storedMotion(localStorage));
  // On a Mac the window draws under its title bar; the bar keeps room for the window buttons.
  if (capabilities().overlayTitleBar) document.documentElement.dataset['titlebar'] = 'overlay';
  // The bar shows icons only below the compact breakpoint or when the operator collapses it; set before the first
  // render so the width never changes after load.
  let nav = $state({ iconOnly: false, compact: false });
  const startsCollapsed = storedCollapsed(localStorage);
  let collapsed = $state(startsCollapsed);
  const iconOnlyWatch = watchIconOnly(document.documentElement, window, startsCollapsed, (now) => {
    nav = now;
  });
  $effect(() => iconOnlyWatch.stop);

  /** Collapses or expands the bar: its width changes at once and the page glides to its new edge (NAV-006, MOT-006). */
  function setCollapsed(next: boolean) {
    const root = document.documentElement;
    const changed = next !== collapsed;
    collapsed = next;
    storeCollapsed(localStorage, next);
    shiftShell(root, () => {
      releaseWidth(root);
      iconOnlyWatch.collapse(next);
    });
    const bar = document.getElementById('main-nav');
    if (changed && !next && bar !== null) void fadeInWords(bar);
  }

  // Decided once, from the page the browser loaded: the operator shell, or a fixed refusal on an origin that cannot be
  // a hub (E-2). Every route but sign-in needs sign-in, so the shell's start guard runs on all of them (API-003, R-111).
  const start = startAt(page.url);
  const shell = start.kind === 'shell' ? startShell(start.address) : null;

  /** The operator shell's parts: the hub and its start guard, the clock, updates, the builder and the session strip. */
  function startShell(address: HubAddress) {
    // The hub address is the stored setting on desktop and the serving origin in a browser (CLI-002, E-2); the token is
    // read from the desktop key store per call, and is none in a browser, where the cookie travels by itself (CLI-003).
    const hub = createHub({ address, token: readAccessToken });
    provideHub(hub);
    guardStart(page.url, hub);
    const clock = createClock(DATA_AGE_REFRESH_MS);
    provideDesktopUpdates();
    // A computer enrolled as a builder builds while the app runs (BLD-026).
    provideDesktopBuilder(hub);

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

{#if shell !== null && showsSignedOutPage(page.url)}
  <!-- Before sign-in the window shows the page alone: no bar, no sessions, no hub status (NAV-009). -->
  <div class="bare">
    <div class="drag" data-tauri-drag-region></div>
    <main>{@render children()}</main>
  </div>
{:else if shell !== null}
  {@const { api, clock, sessions, navigate } = shell}
  <div class="shell">
    <Nav
      pathname={page.url.pathname}
      now={clock.current}
      onnavigate={navigate}
      iconOnly={nav.iconOnly}
      {collapsed}
      canCollapse={!nav.compact}
      ontogglecollapse={() => {
        setCollapsed(!collapsed);
      }}
      oncollapse={setCollapsed}
    />
    <!-- On a Mac the strip above the page moves the window, as the title bar it replaces did. -->
    <div class="drag" data-tauri-drag-region></div>
    <main data-shifts-with-banner>
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
    <UpdateBanner />
  </div>
{:else if start.kind === 'refused'}
  <p class="refused" role="alert">{start.message}</p>
{/if}

<style>
  :global(body) {
    margin: 0;
  }

  .refused {
    margin: 0;
    padding: var(--space-6);
    min-height: 100vh;
    background: var(--surface-sunken);
    color: var(--text);
  }

  .bare {
    min-height: 100vh;
    background: var(--surface-sunken);
    color: var(--text);
    font-family: var(--font-ui);
  }

  .shell {
    display: grid;
    /* The page column may shrink below its content's width, so a narrow window never scrolls sideways (UI-009). */
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    min-height: 100vh;
    background: var(--surface-sunken);
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
    /* The page's side gutter (UR-7); a detail bar runs over it to the page's edges. */
    --page-gutter: var(--space-12);
    padding: var(--space-8) var(--page-gutter) var(--space-16);
  }

  @media (max-width: 1100.98px) {
    main {
      --page-gutter: var(--space-8);
    }
  }

  @media (max-width: 767.98px) {
    main {
      --page-gutter: var(--space-4);
      padding-block-start: var(--space-6);
    }
  }

  .drag {
    display: none;
  }

  /* The window buttons sit over the bar; the strip over the page's top padding drags the window (macOS). */
  :global(:root[data-titlebar='overlay']) .drag {
    display: block;
    position: fixed;
    inset-block-start: 0;
    inset-inline: var(--nav-inline-size) 0;
    block-size: var(--space-6);
    z-index: var(--layer-nav);
  }

  /* With no bar the strip spans the window; the window buttons are drawn over it. */
  :global(:root[data-titlebar='overlay']) .bare .drag {
    inset-inline-start: 0;
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
