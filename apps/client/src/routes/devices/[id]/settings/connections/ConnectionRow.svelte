<script lang="ts">
  import type { Snippet } from 'svelte';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import type { SaveMark as Mark } from '../draft.svelte';

  interface Props {
    /** `ssh`, `rdp` or `vnc`: the row's anchor for a header button (DEV-022). */
    protocol: string;
    name: string;
    icon: IconName;
    /** One line after the name, such as "Port 22 · root · password". */
    summary: Snippet;
    /** Whether the protocol is on, as shown. */
    enabled: boolean;
    /** False shows "Set up" in place of the switch (D17). */
    setUp: boolean;
    open: boolean;
    mark: Mark | undefined;
    onToggle: (next: boolean) => void;
    onSetUp: () => void;
    /** The protocol's fields, shown while the row is open. */
    children: Snippet;
  }

  let { protocol, name, icon, summary, enabled = $bindable(), setUp, open = $bindable(), mark, onToggle, onSetUp, children }: Props = $props();
  const bodyId = $derived(`device-connection-${protocol}`);
</script>

<!-- One protocol, a hairline row (UR-7, D17): its name and summary open its fields; the switch at the end turns it on or
     off at once. -->
<div class="connection" class:off={!enabled} data-connection={protocol}>
  <div class="head">
    <button type="button" tabindex="0" class="toggle" aria-expanded={open} aria-controls={bodyId} onclick={() => (open = !open)}>
      <span class="chevron" class:turned={open}><Icon name="carbon:chevron-right" size="sm" /></span>
      <Icon name={icon} size="sm" />
      <span class="name">{name}</span>
      <span class="summary">{@render summary()}</span>
    </button>
    <SaveMark {mark} />
    {#if setUp}
      <Switch label={name} bind:checked={enabled} onchange={onToggle} />
    {:else}
      <FormButton size="sm" icon="carbon:add" onclick={onSetUp}>Set up</FormButton>
    {/if}
  </div>
  {#if open}
    <div class="body" id={bodyId}>{@render children()}</div>
  {/if}
</div>

<style>
  .connection {
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-block-size: var(--space-12);
    padding: 0 var(--space-1);
  }

  .toggle {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
    block-size: var(--space-12);
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: start;
    cursor: pointer;
  }

  .toggle:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .chevron {
    display: inline-flex;
    color: var(--text-quiet);
    transition: rotate var(--duration-base) var(--ease-enter);
  }

  .turned {
    rotate: 90deg;
  }

  .toggle:hover .chevron {
    color: var(--text);
  }

  .name {
    font-weight: var(--weight-medium);
    white-space: nowrap;
  }

  /* An off protocol's name and icon step back; its chevron and summary keep their own colours. */
  .off .toggle {
    color: var(--text-tertiary);
  }

  .summary {
    min-inline-size: 0;
    overflow: hidden;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .body {
    padding: var(--space-1) var(--space-1) var(--space-5) calc(var(--space-8) - var(--border-focus));
  }

  @media (max-width: 767.98px) {
    .summary {
      display: none;
    }

    .body {
      padding-inline-start: var(--space-1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron {
      transition: none;
    }
  }
</style>
