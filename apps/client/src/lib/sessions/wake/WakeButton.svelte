<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import type { WakeApi } from './api';
  import { wakeText } from './wake';

  interface Props {
    /** The device to wake. */
    deviceId: string;
    /** The wake call; `wakeApi(hub)` in the app, a fake in tests. */
    api: WakeApi;
    /** Set when the caller knows a wake cannot help, for example while the device is Up. */
    disabled?: boolean;
  }

  const { deviceId, api, disabled = false }: Props = $props();

  let busy = $state(false);
  let text = $state('');

  async function wake(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      text = wakeText(await api.wakeDevice(deviceId));
    } finally {
      busy = false;
    }
  }
</script>

<!-- RA-031: a manual wake; the status line holds a fixed block so the text causes no layout shift (MOT-007). -->
<div class="wake">
  <button type="button" onclick={wake} disabled={disabled || busy} aria-busy={busy} {@attach press()}>
    <Icon name="nf-md-lan_connect" />
    <span>Wake</span>
  </button>
  <p class="status" role="status">
    {#key text}
      <span transition:crossFade>{text}</span>
    {/key}
  </p>
</div>

<style>
  .wake {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }

  button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--surface-base);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:active {
    background: var(--accent-pressed);
  }

  button:disabled {
    opacity: var(--opacity-disabled);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .status {
    /* Room for two lines, and both layers of a cross-fade in one cell, so a new text moves nothing. */
    display: grid;
    margin: 0;
    min-block-size: calc(2 * var(--leading-normal) * 1em);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .status > span {
    grid-area: 1 / 1;
  }
</style>
