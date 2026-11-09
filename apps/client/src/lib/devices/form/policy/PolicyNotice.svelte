<script lang="ts">
  import type { PolicyDraft } from './draft.svelte';

  const { policy }: { policy: PolicyDraft } = $props();
</script>

<!-- Outside the Sessions disclosure, so it is seen and announced when that is closed; the line is always there. -->
<p role="status">{policy.notice}</p>

<style>
  p {
    margin: 0;
    min-block-size: var(--target-min-desktop);
    color: var(--status-down);
    font-size: var(--text-xs);
  }
</style>
