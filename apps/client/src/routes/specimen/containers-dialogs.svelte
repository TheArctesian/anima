<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';

  type Size = 'sm' | 'md' | 'lg';
  const SIZES: readonly { size: Size; title: string; line: string }[] = [
    { size: 'sm', title: 'Delete nas?', line: 'Its credentials, checks and history go too.' },
    { size: 'md', title: 'Add device', line: 'A form: fields in the body, actions in the footer.' },
    { size: 'lg', title: 'Session recording', line: 'A wide task. The body scrolls and fades where it is cut.' },
  ];
  let open = $state<Size | null>(null);
  const filler = Array.from({ length: 24 }, (_, index) => index + 1);
</script>

<div class="stack">
  <h3>Dialogs: sm confirm, md form, lg wide; focus is trapped and returns to the button</h3>
  <div class="row">
    {#each SIZES as dialog (dialog.size)}
      <FormButton onclick={() => (open = dialog.size)}>Open {dialog.size}</FormButton>
    {/each}
  </div>
</div>

{#each SIZES as dialog (dialog.size)}
  <Modal open={open === dialog.size} title={dialog.title} preset={dialog.size} dismissible onClose={() => (open = null)}>
    <p>{dialog.line}</p>
    {#if dialog.size === 'lg'}
      <ol>{#each filler as line (line)}<li>Recording line {line}</li>{/each}</ol>
    {/if}
    {#snippet footer()}
      <FormButton onclick={() => (open = null)}>Cancel</FormButton>
      <FormButton tone={dialog.size === 'sm' ? 'danger' : 'primary'} onclick={() => (open = null)}>{dialog.size === 'sm' ? 'Delete nas' : 'Save'}</FormButton>
    {/snippet}
  </Modal>
{/each}

<style>
  .stack {
    display: grid;
    gap: var(--space-3);
  }

  h3 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  .row {
    display: flex;
    gap: var(--space-2);
  }

  p {
    margin: 0;
  }
</style>
