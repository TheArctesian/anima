<script lang="ts">
  import type { DeviceCheck } from '$lib/checks/api';
  import { statusLabel } from '$lib/devices/card/card';
  import type { HubError, HubResult } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { checkRowText } from './rows';

  interface Props {
    check: DeviceCheck;
    /** Its heartbeat history is open below the list. */
    open: boolean;
    onHistory: () => void;
    /** CHK-005: pause (true) or resume (false). */
    onPause: (paused: boolean) => Promise<HubResult<unknown>>;
    onEdit: () => void;
    onDelete: () => void;
  }

  const { check, open, onHistory, onPause, onEdit, onDelete }: Props = $props();
  const text = $derived(checkRowText(check));
  let pausing = $state(false);
  let failure = $state<HubError | null>(null);

  async function togglePause(): Promise<void> {
    if (pausing) return;
    pausing = true;
    const result = await onPause(!check.paused);
    pausing = false;
    failure = result.ok ? null : result.error;
  }
</script>

<li class="row" class:paused={check.paused}>
  <button type="button" class="name" title={check.name} aria-expanded={open} onclick={onHistory} {@attach press()}>
    <span>{check.name}</span>
  </button>
  <span class="facts">
    <span class="type" title={text.type}>{text.type}</span>
    <!-- A reading changes with a cross-fade, in place (MOT-008, Status change). -->
    <span class="reading">
      {#key check.status}<span transition:crossFade>{check.paused ? 'Paused' : statusLabel(check.status)}</span>{/key}
    </span>
    <span class="reading ms">
      {#key text.responseTime}<span transition:crossFade>{text.responseTime}</span>{/key}
    </span>
    <span class="expiry" title={text.expiry ?? undefined}>{text.expiry ?? ''}</span>
  </span>
  <span class="actions">
    <!-- The name starts with the visible text, Pause or Resume (label in name). -->
    <button
      type="button"
      aria-label="{check.paused ? 'Resume' : 'Pause'} {check.name}"
      aria-busy={pausing}
      onclick={() => void togglePause()}
      {@attach press()}
    >
      <Icon name={check.paused ? 'carbon:play' : 'carbon:pause'} size="sm" />{check.paused ? 'Resume' : 'Pause'}
    </button>
    <button type="button" aria-label="Edit {check.name}" onclick={onEdit} {@attach press()}><Icon name="carbon:edit" size="sm" /></button>
    <button type="button" aria-label="Delete {check.name}" onclick={onDelete} {@attach press()}><Icon name="carbon:trash-can" size="sm" /></button>
  </span>
  {#if failure !== null}<p class="failure" role="alert">{failure.message}</p>{/if}
</li>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr) minmax(0, 2fr) auto;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-touch);
    border-block-end: var(--border-thin) solid var(--border-strong);
  }

  .paused {
    color: var(--text-secondary);
  }

  .reading {
    display: grid;
  }

  .reading > span {
    grid-area: 1 / 1;
  }

  .ms {
    font-variant-numeric: tabular-nums;
  }

  .type,
  .expiry {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  /* One line in its column, so a long reading never changes the row's height (MOT-007). */
  .expiry {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* One line in its cell at every width: a long reading is cut with an ellipsis, never wrapped (MOT-007). */
  .type,
  .reading > span {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Wide: the facts are the row's own columns. */
  .facts {
    display: contents;
  }

  /*
   * Below the wide breakpoint the row is three lines, each reserved in every row so no row differs in height (MOT-007): the
   * name and the actions at their natural width, then the type, the status and the latency, then the expiry on a line of its
   * own. The breakpoint is the medium edge of `$lib/motion/tokens`, less 0.02px.
   */
  @media (max-width: 1100.98px) {
    .row {
      grid-template-columns: minmax(0, 1fr) auto;
      padding-block: var(--space-1);
    }

    .name,
    .actions {
      grid-row: 1;
    }

    .actions {
      grid-column: 2;
    }

    .facts {
      display: grid;
      grid-row: 2;
      grid-column: 1 / -1;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: var(--space-1) var(--space-2);
    }

    .type,
    .reading,
    .expiry {
      min-inline-size: 0;
      min-block-size: 1lh;
    }

    .expiry {
      grid-column: 1 / -1;
    }
  }

  .actions {
    display: flex;
    gap: var(--space-1);
  }

  .failure {
    grid-column: 1 / -1;
    margin: 0;
    color: var(--status-down);
  }

  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-hover);
  }

  .name {
    min-inline-size: 0;
    border-color: transparent;
    background: none;
    text-align: start;
  }

  .name span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
