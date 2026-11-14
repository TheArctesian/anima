<script lang="ts">
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { applyPalette, PALETTES, storedPalette, storePalette, type Palette } from './palette';

  let palette = $state<Palette>(storedPalette(localStorage));
  let problem = $state<string | null>(null);

  function choose(chosen: Palette): void {
    palette = chosen;
    applyPalette(document.documentElement, chosen);
    try {
      storePalette(localStorage, chosen);
      problem = null;
    } catch {
      problem = 'Anima could not save this choice on this device; it lasts until the app restarts.';
    }
  }
</script>

<SettingRow label="Colours">
  <Segmented label="Colours" options={PALETTES.map((each) => ({ value: each.id, label: each.label }))} value={palette} onchange={choose} />
  {#snippet message()}
    <!-- Only a refused save shows a message: the row stays one line until then. -->
    {#if problem !== null}<p role="alert">{problem}</p>{/if}
  {/snippet}
</SettingRow>
