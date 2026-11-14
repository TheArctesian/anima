<script lang="ts">
  import { describedBy, inputId } from '$lib/devices/form/form/errors';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import type { SettingsDraft } from '../draft.svelte';
  import { setValueAt, valueAt } from '../groups';

  interface Props {
    draft: SettingsDraft;
    /** The device field the row edits. */
    key: 'name' | 'host' | 'kind' | 'macAddress';
    label: string;
    /** The muted line under the label. */
    about?: string | undefined;
    /** Machine text, such as an address: the monospace font. */
    mono?: boolean;
    placeholder?: string | undefined;
    /** The id of a `datalist` that suggests values. */
    list?: string | undefined;
  }

  const { draft, key, label, about, mono = false, placeholder, list }: Props = $props();
  const id = $derived(inputId(key));
  const error = $derived(draft.errorOf(key));
</script>

{#snippet status()}<SaveMark mark={draft.marks[key]} />{/snippet}
{#snippet hint()}<p class="hint" id="{id}-hint">{about}</p>{/snippet}
{#snippet message()}<p role="alert" id="{id}-error">{error?.message}</p>{/snippet}

<!-- A text setting saves when it is left or Return is pressed, so a half-typed value never saves (D17). -->
<SettingRow {label} for={id} wide {status} hint={about === undefined ? undefined : hint} message={error === undefined ? undefined : message}>
  <Input
    {id}
    {mono}
    {placeholder}
    {list}
    invalid={error !== undefined}
    aria-describedby={describedBy(key, about !== undefined, error !== undefined)}
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
</SettingRow>
