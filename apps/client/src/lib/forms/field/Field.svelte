<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import HelpTip from '$lib/overlay/helptip/HelpTip.svelte';
  import { controlLinks, type ControlLinks } from './field';

  interface Props {
    label: string;
    /** Stacked puts the label above the control; inline puts it in a column beside the control, as a settings row does. */
    layout?: 'stacked' | 'inline';
    /** A format the operator needs to fill the field correctly. It stays visible (UI-028). */
    constraint?: string;
    /** What is wrong with the value now. Empty or missing means nothing is wrong. */
    error?: string;
    /** A default, an explanation or an allowed range: the HelpTip next to the label. */
    help?: Snippet;
    /** The control. Spread `link` onto it: its id, description and validity come from this field. */
    children: Snippet<[link: ControlLinks]>;
  }

  const { label, layout = 'stacked', constraint, error, help, children }: Props = $props();
  const id = $props.id();
  const link = $derived(controlLinks(id, constraint !== undefined, error !== undefined && error !== ''));
</script>

<div class="field {layout}">
  <div class="head">
    <label for={link.id}>{label}</label>
    {#if help}<HelpTip {label}>{@render help()}</HelpTip>{/if}
  </div>
  <div class="body">
    {@render children(link)}
    {#if constraint !== undefined}<p class="constraint" id="{id}-constraint">{constraint}</p>{/if}
    <!-- The region is always there, so a screen reader hears an error when it appears (WCAG 4.1.3). -->
    <div role="alert">
      {#if error !== undefined && error !== ''}
        <p class="error" id="{id}-error"><Icon name="carbon:warning" size="sm" />{error}</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .field {
    display: grid;
    gap: var(--space-1);
  }

  .inline {
    grid-template-columns: minmax(8rem, 14rem) minmax(0, 1fr);
    column-gap: var(--space-4);
    align-items: start;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    /* The label sits on the first control's line, so a tall body does not push it down. */
    min-block-size: var(--control-md);
  }

  .stacked .head {
    min-block-size: 0;
  }

  label {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .body {
    display: grid;
    gap: var(--space-1);
    justify-items: start;
  }

  .constraint {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .error {
    display: flex;
    align-items: baseline;
    gap: var(--space-1);
    margin: 0;
    color: var(--status-down-text);
    font-size: var(--text-sm);
  }
</style>
