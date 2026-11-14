<script lang="ts">
  import DeviceBar from './header/DeviceBar.svelte';
  import DetailBody from './detail/DetailBody.svelte';
  import DeviceMissing from './detail/DeviceMissing.svelte';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();
</script>

{#if data.id === null}
  <section class="detail">
    <!-- A malformed ID: only the back link and the not-found state, and no request goes to the hub (D-14). -->
    <DeviceBar device={null} checks={null} onConnect={() => undefined} onSettings={() => undefined} />
    <DeviceMissing />
  </section>
{:else}
  <DetailBody id={data.id} />
{/if}

<style>
  .detail {
    min-inline-size: 0;
  }
</style>
