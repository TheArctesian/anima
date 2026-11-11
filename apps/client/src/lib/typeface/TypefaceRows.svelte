<script lang="ts">
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { applyTypefaces, storedTypefaces, storeTypeface, TEXT_ROLES, TYPEFACES, type TextRole, type Typeface, type TypefaceChoice } from './typeface';

  const id = $props.id();
  let choice = $state<TypefaceChoice>(storedTypefaces(localStorage));
  let problem = $state<string | null>(null);

  function choose(role: TextRole, typeface: Typeface): void {
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
  <SettingRow label={role.label} for="{id}-{role.id}">
    <select id="{id}-{role.id}" value={choice[role.id]} onchange={(event) => { choose(role.id, event.currentTarget.value as Typeface); }}>
      {#each TYPEFACES as typeface (typeface.id)}
        <option value={typeface.id}>{typeface.label}</option>
      {/each}
    </select>
    {#snippet message()}
      <!-- One message for both fonts, under the last row. -->
      {#if problem !== null && index === TEXT_ROLES.length - 1}<p role="alert">{problem}</p>{/if}
    {/snippet}
  </SettingRow>
{/each}
