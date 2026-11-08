<script lang="ts">
  import Modal from '../Modal.svelte';
  import FooterActions from './FooterActions.svelte';

  // Test support: an open modal whose form can be taken out while the modal stays.
  interface Props {
    form: boolean;
  }

  const { form }: Props = $props();
</script>

<Modal open title="Form" onClose={() => undefined}>
  {#if form}<FooterActions />{/if}
</Modal>
