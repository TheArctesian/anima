<script lang="ts">
  import SignOutRow from '$lib/access/SignOutRow.svelte';
  import TypefaceRows from '$lib/typeface/TypefaceRows.svelte';
  import DesktopRows from '$lib/updates/DesktopRows.svelte';
</script>

<h1>Settings</h1>

<SignOutRow />

<TypefaceRows />

<DesktopRows />

<style>
  /* The title's line height is reserved from tokens, so the font load cannot move it (WP-11 H-7, R-042). */
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-2xl) * var(--leading-tight));
  }
</style>
