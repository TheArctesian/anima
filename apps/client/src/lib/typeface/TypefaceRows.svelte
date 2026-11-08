<script lang="ts">
  import { applyTypefaces, storedTypefaces, storeTypeface, TEXT_ROLES, TYPEFACES, type TextRole, type Typeface, type TypefaceChoice } from './typeface';

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

<section aria-label="Fonts">
  {#each TEXT_ROLES as role (role.id)}
    <label>
      <span>{role.label}</span>
      <select value={choice[role.id]} onchange={(event) => { choose(role.id, event.currentTarget.value as Typeface); }}>
        {#each TYPEFACES as typeface (typeface.id)}
          <option value={typeface.id}>{typeface.label}</option>
        {/each}
      </select>
    </label>
  {/each}
  {#if problem !== null}
    <p role="alert">{problem}</p>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    align-items: start;
    gap: var(--space-2);
    margin-block-start: var(--space-6);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-md);
  }

  /* The label's line is reserved from tokens, so a font change or load cannot move the control (MOT-007). */
  span {
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    line-height: var(--leading-tight);
  }

  select {
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--select-chevron-room) 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background-color: var(--surface);
    color: var(--text);
    font: inherit;
  }

  select:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
