<script lang="ts">
  import { asset, resolve } from '$app/paths';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import LiveSessions from '$lib/sessions/live/LiveSessions.svelte';
  import { press } from '$lib/motion/roles/press';
  import { fade } from '$lib/motion/roles/transitions';
  import { watchHubBanner } from './banner-shift';
  import { DEVICES, SETTINGS, isActive, statusLabel, unreachableBanner, type NavDestination } from './nav';

  interface Props {
    /** The current route, for the active item. */
    pathname: string;
    /** The time the data age is measured against. */
    now: Date;
    /** Called when the operator picks a destination, even the current page (it leaves a session tab). */
    onnavigate?: () => void;
    /** The bar shows icons only, so each item needs its words as a tooltip (UI-015). */
    iconOnly?: boolean;
  }

  const { pathname, now, onnavigate, iconOnly = false }: Props = $props();
  const hub = useHub();
  const connected = $derived(hub.status.current.connection === 'connected');
  const banner = $derived(unreachableBanner(hub.status.current, hub.address.origin, now));

  // Below the compact breakpoint the bar and the page make room for the strip; the root attribute tells their styles (NET-004).
  const strip = watchHubBanner(document.documentElement);
  $effect(() => {
    strip.show(banner !== null);
  });
  $effect(() => strip.stop);
</script>

<!-- tabindex="0": WebKit (Safari, WKWebView) skips plain links when Tab is pressed unless they are focusable (NAV-005). -->
{#snippet destination(item: NavDestination)}
  <a
    class="item"
    href={resolve(item.href)}
    tabindex="0"
    title={iconOnly ? item.label : undefined}
    aria-current={isActive(item, pathname) ? 'page' : undefined}
    onclick={onnavigate}
    {@attach press()}
  >
    <Icon name={item.icon} />
    <span class="words">{item.label}</span>
  </a>
{/snippet}

<nav aria-label="Main" data-shifts-with-banner>
  <div class="brand">
    <img src={asset('/logo.png')} alt="" width="48" height="48" />
    <span class="words">Anima</span>
  </div>
  {@render destination(DEVICES)}
  <LiveSessions {hub} {iconOnly} />
  <!-- Always rendered, so a screen reader announces the text when it appears (NET-004). -->
  <div class="notice" role="status" data-holds-banner>
    {#if banner !== null}
      <div class="banner" transition:fade title="{banner.title}. {banner.origin}. {banner.advice}. {banner.age}">
        <span class="alert"><Icon name="nf-md-alert" /></span>
        <strong>{banner.title}</strong>
        <span class="origin">{banner.origin}</span>
        <span>{banner.advice}</span>
        <span>{banner.age}</span>
      </div>
    {/if}
  </div>
  <hr />
  <p class="status" title={iconOnly ? statusLabel(hub.status.current) : undefined}>
    <span class="dot" class:connected aria-hidden="true"></span><span class="words">{statusLabel(hub.status.current)}</span>
  </p>
  {@render destination(SETTINGS)}
</nav>

<style>
  /* Fixed width and fixed item height: the banner and the font swap move nothing (MOT-007). */
  nav {
    position: sticky;
    top: 0;
    box-sizing: border-box;
    inline-size: var(--nav-inline-size);
    block-size: 100vh;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    background: var(--material-chrome-bg);
    backdrop-filter: blur(var(--material-chrome-blur)) saturate(var(--material-chrome-saturate));
    border-right: var(--border-thin) solid var(--border);
    z-index: var(--layer-nav);
  }

  .item {
    box-sizing: border-box;
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--target-min-touch);
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text);
    line-height: var(--leading-tight);
    text-decoration: none;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .brand {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-2);
    padding-bottom: var(--space-3);
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
  }

  .brand img {
    inline-size: var(--space-12);
    block-size: var(--space-12);
  }

  .item:not([aria-current='page']):hover {
    background: var(--surface-raised);
  }

  .item:not([aria-current='page']):active {
    transition-duration: var(--duration-press);
    background: var(--border);
  }

  /*
   * --accent text on the chrome is 5.0:1 (nord8 on nord1); an accent bar marks the page instead of a tint (NAV-002).
   * The active item gets no hover or press background: on nord2 it would be 4.31:1 and on nord3 3.69:1.
   */
  .item[aria-current='page'] {
    color: var(--accent);
    box-shadow: inset var(--border-focus) 0 0 var(--accent);
  }

  .item:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .notice {
    flex: none;
  }

  /*
   * Connected is a filled dot and anything else a ring, so the state is a shape and not a colour alone (UI-003). The
   * outline keeps 3:1 against the bar; under forced colours the system colours take over.
   */
  .dot {
    display: none;
    flex: none;
    box-sizing: border-box;
    inline-size: var(--space-3);
    block-size: var(--space-3);
    border: var(--border-thin) solid var(--text);
    border-radius: var(--radius-pill);
  }

  .dot.connected {
    background: var(--status-up);
  }

  @media (forced-colors: active) {
    .dot {
      /* stylelint-disable-next-line scale-unlimited/declaration-strict-value -- forced colours use system colours, not tokens */
      border-color: CanvasText;
    }

    .dot.connected {
      /* stylelint-disable-next-line scale-unlimited/declaration-strict-value -- forced colours use system colours, not tokens */
      background: Highlight;
    }
  }

  .alert {
    display: none;
  }

  /*
   * Below the compact breakpoint the bar shows icons only. The words stay in the page as each control's accessible name
   * (UI-015); the width switches at once, never animated (MOT-006).
   */
  :global(:root[data-nav='icons']) .words {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  :global(:root[data-nav='icons']) .item {
    justify-content: center;
    padding: 0;
  }

  :global(:root[data-nav='icons']) .brand {
    justify-content: center;
  }

  :global(:root[data-nav='icons']) .brand img {
    inline-size: var(--target-min-touch);
    block-size: var(--target-min-touch);
  }

  :global(:root[data-nav='icons']) .status {
    display: flex;
    justify-content: center;
    padding: 0;
  }

  :global(:root[data-nav='icons']) .dot {
    display: block;
  }

  /* No blur: a backdrop filter would make the bar the containing block of the fixed strip below. */
  :global(:root[data-nav='icons']) nav {
    backdrop-filter: none;
  }

  /*
   * When the hub cannot be reached the bar's banner is a strip across the top of the window, one line each for the title,
   * the hub address (cut with an ellipsis; the whole text is the tooltip), the advice and the data age. Its size is the
   * same whatever the text, and the bar and the page make room for it, so it covers no control (NET-004, UI-013). They move into that room by FLIP and the strip fades.
   */
  :global(:root[data-nav='icons']) .banner {
    position: fixed;
    inset-block-start: 0;
    inset-inline: 0;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    grid-auto-rows: calc(var(--text-sm) * var(--leading-tight));
    align-items: center;
    gap: 0 var(--space-2);
    block-size: var(--nav-banner-block-size);
    padding: var(--space-2) var(--space-3);
    overflow: hidden;
    border-block-end: var(--border-thin) solid var(--border);
    border-radius: 0;
    background: var(--surface-raised);
    box-shadow: var(--elevation-2);
    white-space: nowrap;
  }

  :global(:root[data-nav='icons']) .banner > * {
    grid-column: 1 / -1;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(:root[data-nav='icons']) .banner > .alert {
    grid-area: 1 / 1;
    overflow: visible;
  }

  :global(:root[data-nav='icons']) .banner > strong {
    grid-area: 1 / 2;
  }

  :global(:root[data-nav='icons'][data-hub-banner]) nav {
    padding-block-start: calc(var(--space-3) + var(--nav-banner-block-size));
  }

  :global(:root[data-nav='icons']) .alert {
    display: inline-flex;
  }

  hr {
    inline-size: 100%;
    margin: auto 0 var(--space-2);
    border: 0;
    border-top: var(--border-thin) solid var(--border);
  }

  /* One line with a height set from tokens, so the font swap cannot move the divider (MOT-007). */
  .status {
    flex: none;
    margin: 0;
    padding: 0 var(--space-3);
    block-size: calc(var(--text-sm) * var(--leading-tight));
    line-height: var(--leading-tight);
    overflow: hidden;
    white-space: nowrap;
    color: var(--text);
    font-size: var(--text-sm);
  }

  .banner {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--status-down-tint);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
    overflow-wrap: anywhere;
  }
</style>
