<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { Tab } from '../tabs';
  import TabMenu from './TabMenu.svelte';
  import { tabElementId } from './tablist';

  interface Props {
    tab: Tab;
    selected: boolean;
    /** Whether this tab is the strip's one stop in the Tab order (roving tabindex). */
    focusable: boolean;
    /** The id of the panel this tab shows. */
    panel: string;
    onselect: () => void;
    onclose: (event: MouseEvent) => void;
    /** Present only where a window can take the session (D-13); without it no menu is offered. */
    onpopout?: () => void;
  }

  const { tab, selected, focusable, panel, onselect, onclose, onpopout }: Props = $props();
  const closeLabel = $derived(`Close ${tab.deviceName} ${tab.protocol} session`);
</script>

<button
  type="button"
  class="tab"
  role="tab"
  id={tabElementId(tab.id)}
  aria-selected={selected}
  aria-controls={panel}
  tabindex={focusable ? 0 : -1}
  data-tab
  onclick={onselect}
  {@attach press()}
>
  <Icon name="nf-md-console" size="sm" />
  <span class="name">{tab.deviceName}</span>
  <span class="protocol">{tab.protocol}</span>
</button>
{#if onpopout !== undefined}
  <TabMenu label={`More for ${tab.deviceName} ${tab.protocol} session`} ready={tab.sessionId !== null} {onpopout} />
{/if}
<button type="button" class="close" aria-label={closeLabel} title={closeLabel} data-no-drag onclick={onclose}>
  <Icon name="nf-md-close" size="sm" />
</button>

<style>
  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--target-min-desktop);
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    cursor: default;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .tab {
    padding: 0 var(--space-2) 0 var(--space-3);
    border-radius: var(--radius-md) 0 0 var(--radius-md);
    min-inline-size: 0;
  }

  .tab[aria-selected='true'] {
    box-shadow: inset 0 calc(var(--border-focus) * -1) 0 var(--accent);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .protocol {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .close {
    justify-content: center;
    inline-size: var(--target-min-desktop);
    border-radius: 0 var(--radius-md) var(--radius-md) 0;
  }

  button:hover {
    background: var(--surface-raised);
  }

  button:active {
    background: var(--border);
  }
</style>
