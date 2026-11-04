<script lang="ts">
  // PLACEHOLDER for WP-14 T2: `SessionPanes` renders this until `SessionView` (lib/sessions/view/)
  // replaces it. It starts no session and opens no tunnel.
  import type { Tab } from '../tabs';

  const { tab }: { tab: Tab } = $props();
</script>

<section class="placeholder" data-placeholder="session-view">
  <h2>{tab.deviceName}</h2>
  <p>{tab.protocol} session view is not available yet.</p>
</section>

<style>
  .placeholder {
    display: grid;
    gap: var(--space-2);
    align-content: start;
    color: var(--text);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    line-height: var(--leading-tight);
  }

  p {
    margin: 0;
    color: var(--text-muted);
  }
</style>
