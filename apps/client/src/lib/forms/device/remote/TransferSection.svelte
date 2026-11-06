<script lang="ts">
  import type { EditedValues } from '../form/editing';

  interface Props {
    values: EditedValues;
  }

  let { values = $bindable() }: Props = $props();
</script>

<label class="check"><input type="checkbox" bind:checked={values.transfer.upload} />Allow upload</label>
<label class="check"><input type="checkbox" bind:checked={values.transfer.download} />Allow download</label>
<label class="check"><input type="checkbox" bind:checked={values.transfer.copy} />Allow copy from the device</label>
<label class="check"><input type="checkbox" bind:checked={values.transfer.paste} />Allow paste to the device</label>

<style>
  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
</style>
