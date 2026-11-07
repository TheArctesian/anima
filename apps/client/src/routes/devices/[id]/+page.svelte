<script lang="ts">
  import Header from './header/Header.svelte';
  import DetailBody from './device/DetailBody.svelte';
  import DeviceMissing from './device/DeviceMissing.svelte';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();
</script>

{#if data.id === null}
  <section class="detail">
    <!-- A malformed ID: only the back link and the not-found state, and no request goes to the hub (D-14). -->
    <Header device={null} onConnect={() => undefined} onSettings={() => undefined} />
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
