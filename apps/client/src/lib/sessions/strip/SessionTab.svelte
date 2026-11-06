<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { Tab } from '../tabs';
  import { keepFocus } from './menu/anchor';
  import TabMenu from './menu/TabMenu.svelte';
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
  const shortcuts = $derived(onpopout === undefined ? 'Delete' : 'Delete Shift+F10');
  let menuOpen = $state(false);
  let button: HTMLButtonElement | undefined = $state();

  /** The keyboard way to the tab menu (APG): Shift+F10 or the ContextMenu key. */
  function key(event: KeyboardEvent): void {
    if (onpopout === undefined) return;
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      menuOpen = true;
    }
  }
</script>

<button
  type="button"
  class="tab"
  role="tab"
  id={tabElementId(tab.id)}
  aria-selected={selected}
  aria-controls={panel}
  tabindex={focusable ? 0 : -1}
  aria-keyshortcuts={shortcuts}
  data-tab
  bind:this={button}
  onclick={onselect}
  onkeydown={key}
  {@attach press()}
>
  <Icon name="nf-md-console" size="sm" />
  <span class="name">{tab.deviceName}</span>
  <span class="title" title={tab.title}>{tab.title ?? ''}</span>
  <span class="protocol">{tab.protocol}</span>
</button>
{#if onpopout !== undefined}
  <TabMenu
    label={`More for ${tab.deviceName} ${tab.protocol} session`}
    ready={tab.sessionId !== null}
    bind:open={menuOpen}
    {onpopout}
    onreturn={() => button?.focus()}
  />
{/if}
<!-- Not a Tab stop and hidden from the tablist, which owns only tabs: Delete on the tab closes it. -->
<button
  type="button"
  class="close"
  aria-label={closeLabel}
  aria-hidden="true"
  tabindex="-1"
  title={closeLabel}
  data-no-drag
  onpointerdown={keepFocus}
  onmousedown={keepFocus}
  onfocus={() => button?.focus()}
  onclick={onclose}
  {@attach press()}
>
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
    flex: 1 1 auto;
    min-inline-size: 0;
  }

  .tab[aria-selected='true'] {
    box-shadow: inset 0 calc(var(--border-focus) * -1) 0 var(--accent);
  }

  /*
   * The tab has one width (set on its slot), whatever the shell puts in the title. The title takes the room the
   * name, the protocol and the buttons leave, and is the first to give it up; the device name keeps priority
   * (MOT-007, MV-004).
   */
  .title {
    flex: 1 1 0;
    min-inline-size: 0;
    overflow: hidden;
    color: var(--text-muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
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
    flex: none;
    justify-content: center;
    inline-size: var(--target-min-desktop);
    border-radius: 0 var(--radius-md) var(--radius-md) 0;
  }

  /* Forced colours drop the shadow; an underline still marks the selected tab. */
  @media (forced-colors: active) {
    .tab[aria-selected='true'] {
      text-decoration: underline;
      text-decoration-thickness: var(--border-focus);
    }
  }

  button:hover {
    background: var(--surface-raised);
  }

  button:active {
    background: var(--border);
  }
</style>
