<script lang="ts">
  import type { Severity } from '$lib/hub/generated/graphql';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { LogEvent } from '../api';
  import { SEVERITY_CHOICES, clockLabel, deviceLabel, labelOf } from './view';

  interface Props {
    titleId: string;
    open: boolean;
    onToggle: () => void;
    /** The operator's Live and Paused switch (LOG-011). */
    live: boolean;
    onLive: (live: boolean) => void;
    /** Whether the hub's live feed is answering; the Live mark holds still and dims when it is not. */
    connected: boolean;
    /** The newest row, shown on one line while the panel is collapsed. */
    latest: LogEvent | undefined;
    errors: number;
    warnings: number;
    /** Narrows the list to one severity, from its count. */
    onOnly: (severity: Severity) => void;
  }

  const { titleId, open, onToggle, live, onLive, connected, latest, errors, warnings, onOnly }: Props = $props();
  const plural = (count: number, word: string) => `${String(count)} ${word}${count === 1 ? '' : 's'}`;
</script>

<!-- The panel's one line (UR-7, LOG-011): the toggle, the name, the Live switch; collapsed, the newest row; then what needs
     you, as counts that narrow the list. -->
<div class="head">
  <button type="button" class="icon" tabindex="0" aria-expanded={open} onclick={onToggle} {@attach press()}>
    <span class="turn" class:shut={!open}><Icon name="carbon:chevron-down" size="sm" /></span>
    <span class="hidden">{open ? 'Collapse' : 'Expand'}</span>
  </button>
  <span class="mark"><Icon name="carbon:catalog" size="sm" /></span>
  <h2 id={titleId}>Logs</h2>
  <button
    type="button"
    class="live"
    class:off={!connected}
    tabindex="0"
    aria-pressed={live}
    title={live ? 'New events join the list as they come' : 'New events wait until you go back to Live'}
    onclick={() => {
      onLive(!live);
    }}
    {@attach press()}
  >
    {#if live}<span class="dot" aria-hidden="true"></span>Live{:else}<Icon name="carbon:pause" size="sm" />Paused{/if}
  </button>
  {#if !open && latest !== undefined}
    <p class="ticker">
      <time datetime={latest.at}>{clockLabel(latest.at)}</time>
      <span class="severity {latest.severity.toLowerCase()}">{labelOf(SEVERITY_CHOICES, latest.severity)}</span>
      <span class="words">{deviceLabel(latest.deviceName)} · {latest.message}</span>
    </p>
  {/if}
  <span class="tally">
    {#if errors > 0}
      <button type="button" class="count error" tabindex="0" onclick={() => { onOnly('ERROR'); }} {@attach press()}>
        <span class="dot" aria-hidden="true"></span>{plural(errors, 'error')}
      </button>
    {/if}
    {#if warnings > 0}
      <button type="button" class="count warning" tabindex="0" onclick={() => { onOnly('WARNING'); }} {@attach press()}>
        <span class="dot" aria-hidden="true"></span>{plural(warnings, 'warning')}
      </button>
    {/if}
  </span>
</div>

<style>
  .head {
    box-sizing: border-box;
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-3);
    block-size: var(--logs-header-size);
    padding: 0 var(--space-4) 0 var(--space-2);
  }

  button {
    box-sizing: border-box;
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--control-sm);
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--text-xs);
    transition: background-color var(--duration-fast) var(--ease-enter), color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .icon {
    justify-content: center;
    inline-size: var(--target-min-desktop);
    min-block-size: var(--target-min-desktop);
    border-radius: var(--radius-md);
  }

  .turn {
    display: inline-flex;
    transition: rotate var(--duration-base) var(--ease-enter);
  }

  .turn.shut {
    rotate: 0.5turn;
  }

  .hidden {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .mark {
    display: inline-flex;
    color: var(--text-tertiary);
  }

  h2 {
    margin: 0;
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .live {
    color: var(--text-tertiary);
  }

  .live[aria-pressed='false'] {
    color: var(--status-warning-text);
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: currentcolor;
  }

  .live .dot {
    background: var(--status-up);
  }

  .live.off .dot {
    background: var(--status-unknown);
  }

  .ticker {
    display: flex;
    flex: 1;
    align-items: baseline;
    gap: var(--space-3);
    min-inline-size: 0;
    margin: 0;
    font-size: var(--text-sm);
  }

  time {
    flex: none;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .severity {
    flex: none;
    color: var(--text-tertiary);
  }

  .error {
    color: var(--status-down-text);
  }

  .warning {
    color: var(--status-warning-text);
  }

  .words {
    overflow: hidden;
    color: var(--text-secondary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tally {
    display: flex;
    flex: none;
    gap: var(--space-1);
    margin-inline-start: auto;
  }

  @media (max-width: 767.98px) {
    .ticker {
      display: none;
    }
  }
</style>
