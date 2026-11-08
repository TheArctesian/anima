<script lang="ts">
  import { modalFooter } from '../footer';

  // Test support: a form that shows its actions in the modal's footer, as DeviceForm does.
  const footer = modalFooter();
  $effect.pre(() => footer?.show(actions));
</script>

{#snippet actions()}
  <button type="button">Save</button>
{/snippet}
