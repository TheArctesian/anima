<script lang="ts">
  import ExtensionsSection from '$lib/extensions/ExtensionsSection.svelte';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import PageHeader from '$lib/page/PageHeader.svelte';
</script>

<!-- EXT-001: the extensions have a page of their own, beside Settings in the bar (operator, 2026-11-19). -->
<div class="head" {@attach entrance()}>
  <PageHeader title="Extensions" eyebrow={{ text: 'Features beyond devices. Off by default; changes save at once.', icon: 'carbon:grid' }} />
</div>
<div class="page">
  <ExtensionsSection />
</div>

<style>
  .head {
    margin-block-end: var(--space-8);
  }

  .page {
    max-inline-size: calc(var(--space-16) * 11);
    min-inline-size: 0;
  }
</style>
