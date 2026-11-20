<script lang="ts">
  import ExtensionsSection from '$lib/extensions/ExtensionsSection.svelte';
  import { storedView, storeView, type ExtensionsView } from '$lib/extensions/marks';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import PageHeader from '$lib/page/PageHeader.svelte';

  const VIEWS = [
    { value: 'list', label: 'List' },
    { value: 'grid', label: 'Grid' },
  ] as const;
  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let view = $state<ExtensionsView>(storedView(storage));
</script>

<!-- EXT-001: the extensions have a page of their own, beside Settings in the bar, as a list or as a grid; the choice
     stays on this device (operator, 2026-11-19). -->
<div class="head" {@attach entrance()}>
  <PageHeader title="Extensions" eyebrow={{ text: 'Features beyond devices. Off by default; changes save at once.', icon: 'carbon:grid' }}>
    {#snippet actions()}
      <Segmented label="View" size="sm" options={VIEWS} bind:value={view} onchange={(next) => { storeView(storage, next); }} />
    {/snippet}
  </PageHeader>
</div>
<div class="page">
  <ExtensionsSection {view} />
</div>

<style>
  .head {
    margin-block-end: var(--space-6);
  }

  /* The list and the tiles use the width of the page: the control of a row lines up with the control of the view in the
     head, and the line of words of an extension is cut only where the page ends. */
  .page {
    min-inline-size: 0;
  }
</style>
