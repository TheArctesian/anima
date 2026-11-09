<script lang="ts">
  import Modal from '../Modal.svelte';
  import FooterActions from './FooterActions.svelte';

  // Test support: an open modal whose form can be replaced, or taken out, while the modal stays.
  interface Props {
    form: boolean;
    version: number;
  }

  const { form, version }: Props = $props();
</script>

<Modal open title="Form" onClose={() => undefined}>
  {#if form}
    {#key version}<FooterActions label="Save {version}" />{/key}
  {/if}
</Modal>
