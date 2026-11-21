<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { NAME_CHARS } from './clusters';

  interface Props {
    open: boolean;
    /** For example "New group" or "Rename group". */
    title: string;
    /** The name the field starts with: empty for a new group. */
    start?: string;
    /** For example "Make group" or "Rename". */
    saveLabel: string;
    /** Saves the name; the reason when it cannot be used, else null. */
    onSave: (name: string) => string | null;
    onClose: () => void;
  }

  const { open, title, start = '', saveLabel, onSave, onClose }: Props = $props();
  let name = $state('');
  let error = $state('');

  $effect.pre(() => {
    if (open) {
      name = start;
      error = '';
    }
  });

  function save(event: SubmitEvent): void {
    event.preventDefault();
    const problem = onSave(name);
    if (problem === null) onClose();
    else error = problem;
  }
</script>

<Modal {open} {title} {onClose} preset="sm">
  <form id="group-name" onsubmit={save}>
    <Field label="Name" constraint={`1 to ${String(NAME_CHARS)} characters`} {error}>
      {#snippet children(link)}
        <Input {...link} bind:value={name} data-autofocus oninput={() => (error = '')} />
      {/snippet}
    </Field>
  </form>
  {#snippet footer()}
    <FormButton onclick={onClose}>Cancel</FormButton>
    <FormButton tone="primary" type="submit" form="group-name">{saveLabel}</FormButton>
  {/snippet}
</Modal>
