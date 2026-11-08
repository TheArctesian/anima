<script lang="ts" generics="T extends string">
  interface Props {
    label: string;
    value: T;
    options: readonly { readonly value: T; readonly label: string }[];
    error?: string | undefined;
    disabled?: boolean;
  }

  let { label, value = $bindable(), options, error, disabled = false }: Props = $props();
  const id = $props.id();
</script>

<div class="field">
  <label for={id}>{label}</label>
  <select
    {id}
    bind:value
    {disabled}
    aria-invalid={error === undefined ? undefined : 'true'}
    aria-describedby={error === undefined ? undefined : `${id}-error`}
  >
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
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
    color: var(--text-muted);
  }

  select {
    box-sizing: border-box;
    min-inline-size: 0;
    max-inline-size: 100%;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--select-chevron-room) var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background-color: var(--surface);
    color: var(--text);
    font: inherit;
  }

  select:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  [aria-invalid='true'] {
    border-color: var(--status-down);
  }

  .error {
    font-size: var(--text-xs);
    color: var(--status-down);
  }
</style>
