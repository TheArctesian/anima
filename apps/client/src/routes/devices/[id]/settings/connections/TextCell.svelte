<script lang="ts">
  import { describedBy, inputId } from '$lib/devices/form/form/errors';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import type { SettingsDraft } from '../draft.svelte';
  import { setValueAt, valueAt, type SettingKey } from '../groups';

  interface Props {
    draft: SettingsDraft;
    key: SettingKey;
    label: string;
    /** A short line under the field. */
    about?: string | undefined;
    /** A port: a narrow column, digits in the monospace font. */
    port?: boolean;
    placeholder?: string | undefined;
  }

  const { draft, key, label, about, port = false, placeholder }: Props = $props();
  const id = $derived(inputId(key));
  const invalid = $derived(draft.errorOf(key) !== undefined);
</script>

<!-- One field of a protocol: the label above with its save mark, the field under it. It saves when it is left or Return
     is pressed (D17); why it was refused is said once under all the fields. -->
<div class="cell" class:port>
  <span class="label"><label for={id}>{label}</label><SaveMark mark={draft.marks[key]} /></span>
  <Input
    {id}
    mono={port}
    {placeholder}
    {invalid}
    inputmode={port ? 'numeric' : undefined}
    aria-describedby={describedBy(key, about !== undefined, invalid)}
    autocomplete="off"
    spellcheck="false"
    bind:value={() => String(valueAt(draft.values, key)), (next) => { setValueAt(draft.values, key, next); }}
    onblur={() => void draft.commit(key)}
    onkeydown={(event) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        void draft.commit(key);
      }
    }}
  />
  {#if about !== undefined}<small id="{id}-hint">{about}</small>{/if}
</div>

<style>
  .cell {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  .cell > :global(input) {
    inline-size: 100%;
  }

  .label {
    display: flex;
    align-items: baseline;
    min-inline-size: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  small {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
</style>
