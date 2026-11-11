<script lang="ts">
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { crossFadeView } from '$lib/motion/view-transition/cross-fade';
  import { applyTheme, storedTheme, storeTheme, THEMES, type Theme } from './theme';

  const id = $props.id();
  let theme = $state<Theme>(storedTheme(localStorage));
  let problem = $state<string | null>(null);

  function choose(chosen: Theme): void {
    theme = chosen;
    // Only the operator's change fades; the start applies the stored theme at once (5.7.3 Theme change).
    crossFadeView(() => {
      applyTheme(document.documentElement, chosen);
    });
    try {
      storeTheme(localStorage, chosen);
      problem = null;
    } catch {
      problem = 'Anima could not save this choice on this device; it lasts until the app restarts.';
    }
  }
</script>

<SettingRow label="Theme" for={id}>
  <select {id} value={theme} onchange={(event) => { choose(event.currentTarget.value as Theme); }}>
    {#each THEMES as each (each.id)}
      <option value={each.id}>{each.label}</option>
    {/each}
  </select>
  {#snippet message()}
    <!-- Only a refused save shows a message: the row stays one line until then. -->
    {#if problem !== null}<p role="alert">{problem}</p>{/if}
  {/snippet}
</SettingRow>
