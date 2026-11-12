<script lang="ts">
  interface Props {
    label: string;
    checked: boolean;
    hint?: string;
    disabled?: boolean;
  }

  let { label, checked = $bindable(), hint, disabled = false }: Props = $props();
  const id = $props.id();
</script>

<div class="field">
  <label for={id}>
    <input {id} type="checkbox" bind:checked {disabled} aria-describedby={hint === undefined ? undefined : `${id}-hint`} />
    <span>{label}</span>
  </label>
  {#if hint !== undefined}<small id="{id}-hint">{hint}</small>{/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    color: var(--text);
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  small {
    font-size: var(--text-xs);
    color: var(--text-secondary);
  }
</style>
