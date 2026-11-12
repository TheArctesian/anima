<script lang="ts">
  interface Props {
    label: string;
    value: string;
    /** `password` hides secrets; `numeric` opens the number keypad without the spinner of type=number. */
    kind?: 'text' | 'password' | 'numeric' | 'multiline';
    error?: string | undefined;
    hint?: string;
    disabled?: boolean;
    /** Called after each edit by the operator. */
    oninput?: () => void;
  }

  let { label, value = $bindable(), kind = 'text', error, hint, disabled = false, oninput }: Props = $props();
  const id = $props.id();
  const describedBy = $derived(
    [error === undefined ? '' : `${id}-error`, hint === undefined ? '' : `${id}-hint`].filter((part) => part !== '').join(' ') || undefined,
  );
</script>

<div class="field">
  <label for={id}>{label}</label>
  {#if kind === 'multiline'}
    <textarea {id} bind:value {disabled} {oninput} rows="3" aria-invalid={error === undefined ? undefined : 'true'} aria-describedby={describedBy}></textarea>
  {:else}
    <input
      {id}
      bind:value
      {disabled}
      {oninput}
      type={kind === 'password' ? 'password' : 'text'}
      inputmode={kind === 'numeric' ? 'numeric' : undefined}
      autocomplete={kind === 'password' ? 'new-password' : 'off'}
      spellcheck="false"
      aria-invalid={error === undefined ? undefined : 'true'}
      aria-describedby={describedBy}
    />
  {/if}
  {#if hint !== undefined}<small id="{id}-hint">{hint}</small>{/if}
  {#if error !== undefined}<small id="{id}-error" class="error">{error}</small>{/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  label {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  input,
  textarea {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  input:focus-visible,
  textarea:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  [aria-invalid='true'] {
    border-color: var(--status-down);
  }

  small {
    font-size: var(--text-xs);
    color: var(--text-secondary);
  }

  .error {
    color: var(--status-down);
  }
</style>
