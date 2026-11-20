<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { HubResult } from '$lib/hub/hub';

  interface Props {
    open: boolean;
    /** For example "New group" or "Rename group". */
    title: string;
    /** The name the field starts with: empty for a new group. */
    start?: string;
    /** For example "Make group" or "Rename". */
    saveLabel: string;
    onSave: (name: string) => Promise<HubResult<unknown>>;
    onClose: () => void;
  }

  const { open, title, start = '', saveLabel, onSave, onClose }: Props = $props();
  let name = $state('');
  let busy = $state(false);
  let error = $state('');
  /** The hub has the same limit (PRO-059); the field says it before the hub must. */
  const MOST = 60;

  $effect.pre(() => {
    if (open) {
      name = start;
      error = '';
    }
  });

  async function save(event?: SubmitEvent): Promise<void> {
    event?.preventDefault();
    const trimmed = name.trim();
    if (trimmed === '') error = 'Give the group a name.';
    else if (trimmed.length > MOST) error = `Use ${String(MOST)} characters or fewer.`;
    if (busy || error !== '' || trimmed === '') return;
    busy = true;
    const result = await onSave(trimmed);
    busy = false;
    if (result.ok) onClose();
    else error = result.error.message;
  }
</script>

<Modal {open} {title} {onClose} preset="sm">
  <form id="group-name" onsubmit={(event) => void save(event)}>
    <Field label="Name" constraint={`1 to ${String(MOST)} characters`} {error}>
      {#snippet children(link)}
        <Input {...link} bind:value={name} data-autofocus oninput={() => (error = '')} />
      {/snippet}
    </Field>
  </form>
  {#snippet footer()}
    <FormButton onclick={onClose}>Cancel</FormButton>
    <FormButton tone="primary" type="submit" form="group-name" aria-busy={busy} {busy}>{saveLabel}</FormButton>
  {/snippet}
</Modal>
