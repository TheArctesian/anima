<script lang="ts">
  import { asset, resolve } from '$app/paths';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import CollapseToggle from './collapse/CollapseToggle.svelte';
  import ResizeEdge from './collapse/ResizeEdge.svelte';
  import LiveSessions from '$lib/sessions/live/LiveSessions.svelte';
  import { press } from '$lib/motion/roles/press';
  import { capabilities } from '$lib/platform/platform';
  import { fade } from '$lib/motion/roles/transitions';
  import { watchHubBanner } from './banner-shift';
  import { DEVICES, SETTINGS, isActive, optionalDestinations, statusLabel, unreachableBanner, type NavDestination } from './nav';

  interface Props {
    /** The current route, for the active item. */
    pathname: string;
    /** The time the data age is measured against. */
    now: Date;
    /** Called when the operator picks a destination, even the current page (it leaves a session tab). */
    onnavigate?: () => void;
    /** The Projects extension is on: its entry shows, and only then (EXT-002). */
    projectsOn?: boolean;
    /** The bar shows icons only, so each item needs its words as a tooltip (UI-015). */
    iconOnly?: boolean;
    /** The operator collapsed the bar to icons. */
    collapsed?: boolean;
    /** The operator may collapse or expand the bar; below the compact breakpoint it stays icons only. */
    canCollapse?: boolean;
    ontogglecollapse?: () => void;
    /** Called when the operator drags the bar's edge or uses its arrow keys, with the state it should rest in (5.7.4). */
    oncollapse?: (collapsed: boolean) => void;
  }

  const { pathname, now, onnavigate, projectsOn = false, iconOnly = false, collapsed = false, canCollapse = false, ontogglecollapse, oncollapse }: Props =
    $props();
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

<nav id="main-nav" aria-label="Main" data-shifts-with-banner>
  <!-- The top row holds the window buttons on a Mac, and moves the window there (data-tauri-drag-region). -->
  <div class="bar" data-tauri-drag-region>
    {#if canCollapse && ontogglecollapse}
      <CollapseToggle {collapsed} ontoggle={ontogglecollapse} />
    {/if}
  </div>
  <div class="brand">
    <img src={asset('/logo.png')} alt="" width="24" height="24" />
    <span class="words">Anima</span>
  </div>
  {@render destination(DEVICES)}
  <LiveSessions {hub} {iconOnly} />
  {#each optionalDestinations(projectsOn, capabilities().builds) as item (item.href)}
    {@render destination(item)}
  {/each}
  <!-- Always rendered, so a screen reader announces the text when it appears (NET-004). -->
  <div class="notice" role="status" data-holds-banner>
    {#if banner !== null}
      <div class="banner" transition:fade title="{banner.title}. {banner.origin}. {banner.advice}. {banner.age}">
        <span class="alert"><Icon name="carbon:warning-alt" /></span>
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
  {#if canCollapse && oncollapse}
    <ResizeEdge {collapsed} onsettle={oncollapse} />
  {/if}
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
    gap: var(--border-focus);
    padding: var(--space-3) var(--space-2);
    border-right: var(--border-thin) solid var(--border-subtle);
    background: var(--surface-sunken);
    z-index: var(--layer-nav);
  }

  .bar {
    display: flex;
    flex: none;
    align-items: center;
    block-size: var(--target-min-touch);
  }

  /* On a Mac the window buttons sit at the start of the row; the toggle follows them in both widths. */
  :global(:root[data-titlebar='overlay']) nav {
    padding-block-start: var(--space-1);
  }

  :global(:root[data-titlebar='overlay']) .bar {
    padding-inline-start: var(--titlebar-buttons-inline-size);
  }

  :global(:root:not([data-titlebar='overlay'])[data-nav='icons']) .bar {
    justify-content: center;
  }

  /* With the window's own title bar the toggle sits at the end of the brand row, as on the website (UR-7). */
  :global(:root:not([data-titlebar='overlay']):not([data-nav='icons'])) .bar {
    position: absolute;
    inset-block-start: calc(var(--space-3) - var(--space-1) / 2);
    inset-inline-end: var(--space-2);
    block-size: var(--control-sm);
  }

  .item {
    box-sizing: border-box;
    display: flex;
    flex: none;
    align-items: center;
    position: relative;
    gap: var(--space-3);
    block-size: var(--control-md);
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    line-height: var(--leading-tight);
    text-decoration: none;
    transition: background-color var(--duration-fast) var(--ease-enter), color var(--duration-fast) var(--ease-enter);
  }

  .item :global(.icon) {
    color: var(--text-tertiary);
  }

  .brand {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-2) var(--space-6);
    font-size: var(--text-md);
    font-weight: var(--weight-regular);
  }

  .brand img {
    inline-size: var(--space-6);
    block-size: var(--space-6);
  }

  .item:not([aria-current='page']):hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .item:not([aria-current='page']):active {
    transition-duration: var(--duration-press);
    background: var(--surface-hover);
  }

  /* The current page (NAV-002, UR-7): the accent tint, an accent icon and an accent edge bar, a shape as well as a colour (UI-003). */
  .item[aria-current='page'] {
    background: var(--accent-tint);
    color: var(--text);
  }

  .item[aria-current='page'] :global(.icon) {
    color: var(--accent);
  }

  .item[aria-current='page']::before {
    content: '';
    position: absolute;
    inset-block: var(--space-2);
    inset-inline-start: calc(var(--space-2) * -1);
    inline-size: var(--border-focus);
    border-start-end-radius: var(--radius-sm);
    border-end-end-radius: var(--radius-sm);
    background: var(--accent);
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
    -webkit-backdrop-filter: none;
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
    border-block-end: var(--border-thin) solid var(--border-strong);
    border-radius: 0;
    background: var(--surface-overlay);
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
    border-top: var(--border-thin) solid var(--border-subtle);
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
