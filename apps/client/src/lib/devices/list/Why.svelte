<script lang="ts">
  import { resolve } from '$app/paths';
  import type { Beat } from '$lib/checks/heartbeats';
  import Icon from '$lib/icons/Icon.svelte';
  import { reasonOf, type LookDevice } from './look';

  interface Props {
    device: LookDevice;
    /** The device's live beats, oldest first, for how long it has been so. */
    beats: readonly Beat[] | undefined;
    /** Lets the text wrap, for a tile. */
    wrap?: boolean;
  }

  const { device, beats, wrap = false }: Props = $props();
  const reason = $derived(reasonOf(device, beats, Date.now()));
</script>

<!-- Why a device needs a look, how long it has, and the way to its checks (UR-7, D18). -->
{#if reason !== null}
  <span class="why" class:wrap>
    <span class="text" title={reason.text}>{reason.text}</span>
    {#if reason.since !== null}<span class="since">{reason.since}</span>{/if}
    <a href="{resolve('/devices')}/{encodeURIComponent(device.id)}?tab=checks">Open checks<Icon name="carbon:arrow-right" size="sm" /></a>
  </span>
{/if}

<style>
  .why {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    min-inline-size: 0;
    font-size: var(--text-sm);
  }

  .text {
    overflow: hidden;
    color: var(--text-secondary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .since {
    color: var(--text-quiet);
    white-space: nowrap;
  }

  a {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-inline-start: auto;
    color: var(--accent);
    text-decoration: none;
    white-space: nowrap;
  }

  a:hover {
    text-decoration: underline;
    text-underline-offset: var(--space-1);
  }

  a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
    border-radius: var(--radius-sm);
  }

  a :global(.icon) {
    align-self: center;
  }

  .wrap {
    flex-wrap: wrap;
  }

  .wrap .text {
    white-space: normal;
  }
</style>
