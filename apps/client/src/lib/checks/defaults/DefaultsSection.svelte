<script lang="ts">
  import { checkDefaultsApi, type CheckDefaults } from '$lib/checks/api';
  import { defaultsInput, SETTING_FIELDS } from './settings';
  import { messageFor } from '$lib/checks/form/model/errors';
  import { settingsErrors } from '$lib/checks/form/model/validate';
  import { defaultsText } from '$lib/checks/form/model/values';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import type { FieldError } from '$lib/hub/hub';

  const api = checkDefaultsApi(useHub());
  const id = $props.id();
  /** What the operator sees in the four fields; empty while loading. */
  let draft = $state({ intervalSeconds: '', retries: '', retryIntervalSeconds: '', degradedThresholdMs: '' });
  /** `null` while loading or after a failed load. */
  let loaded = $state<CheckDefaults | null>(null);
  let loadFailed = $state(false);
  let errors = $state<FieldError[]>([]);
  let saveFailed = $state(false);
  let busy = $state(false);

  /** Shows `defaults` as the hub holds them, in the fields and as the saved state. */
  function show(defaults: CheckDefaults): void {
    loaded = defaults;
    draft = defaultsText(defaults);
  }

  async function load(): Promise<void> {
    loadFailed = false;
    const result = await api.get();
    if (result.ok) show(result.value);
    else loadFailed = true;
  }

  $effect(() => {
    void load();
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (loaded === null || busy) return;
    saveFailed = false;
    errors = settingsErrors(draft);
    if (errors.length > 0) return;
    busy = true;
    try {
      const result = await api.set(defaultsInput(draft));
      if (result.ok) show(result.value);
      else saveFailed = true;
    } finally {
      busy = false;
    }
  }

  function edited(): void {
    errors = [];
    saveFailed = false;
  }
</script>

<!-- A group, not a form landmark; Enter still submits. -->
<form role="group" aria-label="Default check settings" novalidate onsubmit={submit}>
  <p class="note">New checks start with these settings.</p>
  {#each SETTING_FIELDS as { key, label, bounds } (key)}
    <div class="field">
      <label for="{id}-{key}">{label}</label>
      <input
        id="{id}-{key}"
        type="text"
        inputmode="numeric"
        autocomplete="off"
        spellcheck="false"
        disabled={loaded === null}
        aria-invalid={messageFor(errors, key) === undefined ? undefined : true}
        aria-describedby="{id}-{key}-hint {id}-{key}-problem"
        value={draft[key]}
        oninput={(event) => {
          draft[key] = event.currentTarget.value;
          edited();
        }}
      />
      <!-- The bounds and the message keep their lines whether or not they have text, so nothing below moves (MOT-007, MV-005). -->
      <p class="hint" id="{id}-{key}-hint">Allowed {bounds[0]} to {bounds[1]}.</p>
      <div class="problem" id="{id}-{key}-problem" role="alert">{messageFor(errors, key) ?? ''}</div>
    </div>
  {/each}
  <!-- One block holds the save and load messages, so the section keeps its height at every width. -->
  <div class="note-block">
    {#if loadFailed}
      <p role="alert">Anima could not load the default check settings.</p>
      <FormButton tabindex={0} onclick={load}>Try again</FormButton>
    {:else if saveFailed}
      <p role="alert">Anima could not save the default check settings.</p>
    {/if}
  </div>
  <FormButton tabindex={0} type="submit" aria-label="Save default check settings" aria-disabled={loaded === null || busy}>Save</FormButton>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-block-start: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* The label's line is reserved from tokens, so a font load cannot move what is below it (MOT-007). */
  label {
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  input {
    box-sizing: border-box;
    inline-size: calc(var(--space-8) * 2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  input[aria-invalid='true'] {
    border-color: var(--status-down);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
  }

  .hint,
  .problem {
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .hint {
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
    color: var(--text-muted);
  }

  /* The message may wrap to a second line at a narrow width, so two lines are reserved. */
  .problem {
    min-block-size: calc(var(--text-sm) * var(--leading-tight) * 2);
  }

  .note {
    color: var(--text-muted);
  }

  .note-block {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    /* The failure message may wrap to two lines beside Try again, so two lines are reserved (MOT-007). */
    min-block-size: max(var(--target-min-desktop), calc(var(--text-md) * var(--leading-tight) * 2));
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  .problem:not(:empty),
  .note-block [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
