<script lang="ts">
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { applyMotion, MOTION_CHOICES, storedMotion, storeMotion, type MotionChoice } from './motion-choice';

  const id = $props.id();
  let motion = $state<MotionChoice>(storedMotion(localStorage));
  let problem = $state<string | null>(null);

  function choose(chosen: MotionChoice): void {
    motion = chosen;
    applyMotion(document.documentElement, chosen);
    try {
      storeMotion(localStorage, chosen);
      problem = null;
    } catch {
      problem = 'Anima could not save this choice on this device; it lasts until the app restarts.';
    }
  }
</script>

<SettingRow label="Reduce motion" for={id}>
  <select {id} value={motion} onchange={(event) => { choose(event.currentTarget.value as MotionChoice); }}>
    {#each MOTION_CHOICES as each (each.id)}
      <option value={each.id}>{each.label}</option>
    {/each}
  </select>
  {#snippet message()}
    {#if problem !== null}<p role="alert">{problem}</p>{/if}
  {/snippet}
</SettingRow>
