<script lang="ts">
  // No state yet. The script block gives the page a type for the test that mounts it (svelte-check).
</script>

<h1>Settings</h1>

<!--
  WP-04 stage B mounts the desktop "Updates" row (UPD-006, R-006) here, below the title.
  Nothing renders in this place until then. No Updates code belongs in this page before that stage.
-->

<style>
  /* The title's line height is reserved from tokens, so the font load cannot move it (WP-11 H-7, R-042). */
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-2xl) * var(--leading-tight));
  }
</style>
