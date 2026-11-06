<script lang="ts">
  import { asset, resolve } from '$app/paths';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { DEVICES, SETTINGS, isActive, statusLabel, unreachableBanner, type NavDestination } from './nav';

  interface Props {
    /** The current route, for the active item. */
    pathname: string;
    /** The time the data age is measured against. */
    now: Date;
    /** Called when the operator picks a destination, even the current page (it leaves a session tab). */
    onnavigate?: () => void;
  }

  const { pathname, now, onnavigate }: Props = $props();
  const hub = useHub();
  const banner = $derived(unreachableBanner(hub.status.current, hub.address.origin, now));
</script>

<!-- tabindex="0": WebKit (Safari, WKWebView) skips plain links when Tab is pressed unless they are focusable (NAV-005). -->
{#snippet destination(item: NavDestination)}
  <a
    class="item"
    href={resolve(item.href)}
    tabindex="0"
    aria-current={isActive(item, pathname) ? 'page' : undefined}
    onclick={onnavigate}
    {@attach press()}
  >
    <Icon name={item.icon} />
    <span>{item.label}</span>
  </a>
{/snippet}

<nav aria-label="Main">
  <div class="brand">
    <img src={asset('/logo.png')} alt="" width="48" height="48" />
    <span>Anima</span>
  </div>
  {@render destination(DEVICES)}
  <!-- Always rendered, so a screen reader announces the text when it appears (NET-004). -->
  <div class="notice" role="status">
    {#if banner !== null}
      <div class="banner">
        <strong>{banner.title}</strong>
        <span>{banner.origin}</span>
        <span>{banner.advice}</span>
        <span>{banner.age}</span>
      </div>
    {/if}
  </div>
  <hr />
  <p class="status">{statusLabel(hub.status.current)}</p>
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
    transition: background-color var(--duration-press) var(--ease-enter);
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
