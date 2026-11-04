<script lang="ts">
  import { useSessions } from '../sessions.svelte';
  import SessionViewPlaceholder from './SessionViewPlaceholder.svelte';
  import { panelId, tabElementId } from './tablist';

  const sessions = useSessions();
</script>

<!--
  One panel per open tab, all kept mounted so a hidden session stays alive (D-11); only the
  selected one shows. No motion here: the remote display is never animated (MAT-008).
-->
<div class="panes" hidden={sessions.active === null}>
  {#each sessions.tabs as tab (tab.id)}
    <div
      class="pane"
      role="tabpanel"
      id={panelId(tab.id)}
      aria-labelledby={tabElementId(tab.id)}
      hidden={sessions.active?.id !== tab.id}
    >
      <SessionViewPlaceholder {tab} />
    </div>
  {/each}
</div>

<style>
  /* Shares the page area's grid cell in the root layout, over the pages. */
  .panes {
    grid-area: 1 / 1;
    display: grid;
    background: var(--bg);
  }

  .panes[hidden],
  .pane[hidden] {
    display: none;
  }

  .pane {
    grid-area: 1 / 1;
    min-block-size: 0;
  }
</style>
