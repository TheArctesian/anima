<script lang="ts">
  import { resolve } from '$app/paths';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { DEVICES, SETTINGS, isActive, statusLabel, unreachableBanner, type NavItem } from './nav';

  interface Props {
    /** The current route, for the active item. */
    pathname: string;
    /** The time the data age is measured against. */
    now: Date;
  }

  const { pathname, now }: Props = $props();
  const hub = useHub();
  const banner = $derived(unreachableBanner(hub.status.current, hub.address.origin, now));
</script>

{#snippet destination(item: NavItem)}
  <a
    class="item"
    href={resolve(item.href)}
    aria-current={isActive(item, pathname) ? 'page' : undefined}
    {@attach press()}
  >
    <Icon name={item.icon} />
    <span>{item.label}</span>
  </a>
{/snippet}

<nav aria-label="Main">
  {@render destination(DEVICES)}
  <hr />
  {#if banner !== null}
    <div class="banner" role="status">
      <strong>{banner.title}</strong>
      <span>{banner.origin}</span>
      <span>{banner.advice}</span>
      <span>{banner.age}</span>
    </div>
  {/if}
  <p class="status">{statusLabel(hub.status.current)}</p>
  {@render destination(SETTINGS)}
</nav>

<style>
  nav {
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
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text);
    text-decoration: none;
  }

  .item[aria-current='page'] {
    color: var(--accent);
    background: var(--accent-tint);
  }

  .item:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  hr {
    width: 100%;
    margin: auto 0 var(--space-2);
    border: 0;
    border-top: var(--border-thin) solid var(--border);
  }

  .status {
    margin: 0;
    padding: 0 var(--space-3);
    color: var(--text-muted);
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
  }
</style>
