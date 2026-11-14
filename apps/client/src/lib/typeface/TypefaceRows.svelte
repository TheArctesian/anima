<script lang="ts">
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { applyTypefaces, storedTypefaces, storeTypeface, TEXT_ROLES, type TextRole, type Typeface, type TypefaceChoice } from './typeface';

  let choice = $state<TypefaceChoice>(storedTypefaces(localStorage));
  let problem = $state<string | null>(null);

  function choose<R extends TextRole>(role: R, typeface: Typeface<R>): void {
    choice = { ...choice, [role]: typeface };
    applyTypefaces(document.documentElement, choice);
    try {
      storeTypeface(localStorage, role, typeface);
      problem = null;
    } catch {
      problem = 'Anima could not save this choice on this device; it lasts until the app restarts.';
    }
  }
</script>

{#each TEXT_ROLES as role, index (role.id)}
  <SettingRow label={role.label}>
    <Segmented
      label={role.label}
      options={role.choices.map((typeface) => ({ value: typeface.id, label: typeface.label }))}
      value={choice[role.id]}
      onchange={(typeface) => { choose(role.id, typeface as Typeface<typeof role.id>); }}
    />
    {#snippet message()}
      <!-- One message for both fonts, under the last row. -->
      {#if problem !== null && index === TEXT_ROLES.length - 1}<p role="alert">{problem}</p>{/if}
    {/snippet}
  </SettingRow>
{/each}
