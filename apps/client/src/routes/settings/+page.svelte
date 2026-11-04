<script lang="ts">
  import SignOutRow from '$lib/access/SignOutRow.svelte';
</script>

<h1>Settings</h1>

<SignOutRow />

<!-- WP-04 stage B mounts the desktop "Updates" row here (UPD-006, R-006). -->

<style>
  /* The title's line height is reserved from tokens, so the font load cannot move it (WP-11 H-7, R-042). */
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-2xl) * var(--leading-tight));
  }
</style>
