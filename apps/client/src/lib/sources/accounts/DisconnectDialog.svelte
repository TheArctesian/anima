<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';

  interface Props {
    /** The account being disconnected, named in the dialog. */
    label: string;
    sourceName: string;
    /** Sends the choice; `deleteRecords` false keeps the records the account brought in. */
    onConfirm: (deleteRecords: boolean) => void;
    onCancel: () => void;
  }

  const { label, sourceName, onConfirm, onCancel }: Props = $props();
  const ids = $props.id();
  // Keeping is the choice that loses nothing, so it is the one that is set first.
  let choice = $state<'keep' | 'delete'>('keep');
</script>

<Modal open title="Disconnect {label}" describedBy="{ids}-about" preset="sm" onClose={onCancel}>
  <p id="{ids}-about">
    Disconnect the {sourceName} account {label}. The hub deletes the key it holds and stops reading. Choose what happens to the records it already read.
  </p>
  <fieldset>
    <legend>Records from {label}</legend>
    <label><input type="radio" name="{ids}-records" value="keep" bind:group={choice} /> Keep the records</label>
    <label><input type="radio" name="{ids}-records" value="delete" bind:group={choice} /> Delete the records</label>
  </fieldset>
  {#snippet footer()}
    <FormButton data-autofocus onclick={onCancel}>Cancel</FormButton>
    <FormButton
      tone="danger"
      onclick={() => {
        onConfirm(choice === 'delete');
      }}
    >
      Disconnect {label}
    </FormButton>
  {/snippet}
</Modal>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
    margin: var(--space-3) 0 0;
    padding: 0;
    border: 0;
  }

  legend {
    padding: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  label {
    display: flex;
    min-block-size: var(--target-min-desktop);
    align-items: center;
    gap: var(--space-2);
  }
</style>
