<script lang="ts">
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { applyMotion, MOTION_CHOICES, storedMotion, storeMotion, type MotionChoice } from './motion-choice';

  let motion = $state<MotionChoice>(storedMotion(localStorage));
  let problem = $state<string | null>(null);

  function choose(chosen: MotionChoice): void {
    motion = chosen;
    applyMotion(document.documentElement, chosen);
    try {
      storeMotion(localStorage, chosen);
      problem = null;
    } catch {
      problem = 'Not saved on this device. Lasts until restart.';
    }
  }
</script>

<SettingRow label="Reduce motion">
  <Segmented label="Reduce motion" options={MOTION_CHOICES.map((each) => ({ value: each.id, label: each.label }))} value={motion} onchange={choose} />
  {#snippet message()}
    {#if problem !== null}<p role="alert">{problem}</p>{/if}
  {/snippet}
</SettingRow>
