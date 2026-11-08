<script lang="ts">
  import { duration } from '$lib/motion/tokens';
  import Field from '../Field.svelte';

  interface Props {
    label: string;
    /** What the operator typed now; never a stored value, which the hub does not return (D-17, F-8). */
    value: string;
    problem: string;
    /** `hidden` masks the text (passwords, tokens, keys); `lines` is one value per line (Apprise URLs). */
    shape?: 'line' | 'hidden' | 'lines';
    /** The hub holds a value: an empty field keeps it. */
    stored?: boolean;
    /** Offered for a stored optional secret: checked sends `""`, which removes it (K1). */
    clear?: boolean;
    removable?: boolean;
    hint?: string;
  }

  let { label, value = $bindable(), problem, shape = 'line', stored = false, clear = $bindable(false), removable = false, hint }: Props = $props();

  const shownHint = $derived(stored ? `Stored. Leave empty to keep it.${hint === undefined ? '' : ` ${hint}`}` : hint);

  /**
   * Empties the field the moment it starts to leave. A closing dialog, or a page left while the editor is open,
   * keeps its last content on screen until the fade ends; a secret must not stay in the DOM for that time (F-8).
   */
  function drop(node: HTMLInputElement | HTMLTextAreaElement): { duration: number } {
    node.value = '';
    return { duration: duration.instant };
  }
</script>

<!--
  No `name` attribute: a form that were ever submitted by the browser would put named fields in the URL (F-8).
  No spell check, no autocapitalisation, no autofill: what is typed here goes to the hub and nowhere else.
-->
<Field {label} hint={shownHint} {problem}>
  {#snippet control(attributes)}
    {#if shape === 'lines'}
      <textarea {...attributes} bind:value out:drop|global rows="3" disabled={clear} autocomplete="off" autocapitalize="off" spellcheck="false"></textarea>
    {:else}
      <input
        {...attributes}
        bind:value
        out:drop|global
        type={shape === 'hidden' ? 'password' : 'text'}
        disabled={clear}
        autocomplete={shape === 'hidden' ? 'new-password' : 'off'}
        autocapitalize="off"
        spellcheck="false"
      />
    {/if}
  {/snippet}
</Field>
{#if stored && removable}
  <label class="remove">
    <input
      type="checkbox"
      bind:checked={clear}
      onchange={() => {
        if (clear) value = '';
      }}
    />
    Remove the stored {label.toLowerCase()}
  </label>
{/if}

<style>
  /* The whole row is the target, at least the desktop target size (UI-013). */
  .remove {
    display: flex;
    min-block-size: var(--target-min-desktop);
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .remove input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
