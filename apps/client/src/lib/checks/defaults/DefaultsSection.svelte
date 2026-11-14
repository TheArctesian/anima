<script lang="ts">
  import { tick } from 'svelte';
  import { checkDefaultsApi, type CheckDefaults } from '$lib/checks/api';
  import { defaultsInput, SETTING_FIELDS } from './settings';
  import { messageFor } from '$lib/checks/form/model/errors';
  import { settingsErrors } from '$lib/checks/form/model/validate';
  import { defaultsText } from '$lib/checks/form/model/values';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useActionFeedback } from '$lib/forms/feedback/feedback.svelte';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import type { FieldError } from '$lib/hub/hub';
  import SettingRow from '$lib/settings/SettingRow.svelte';

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
  const saved = useActionFeedback();
  let retrying = $state(false);
  let form: HTMLFormElement | undefined = $state();
  const notice = $derived(loadFailed ? 'Anima could not load the defaults.' : saveFailed ? 'Anima could not save the default check settings.' : '');

  /** Shows `defaults` as the hub holds them, in the fields and as the saved state. */
  function show(defaults: CheckDefaults): void {
    loaded = defaults;
    draft = defaultsText(defaults);
  }

  async function load(): Promise<void> {
    const result = await api.get();
    if (result.ok) {
      show(result.value);
      loadFailed = false;
    } else loadFailed = true;
  }

  /** Try again: the button stays where it is, and aria-disabled, while the hub answers; on success focus goes to the first field. */
  async function retry(): Promise<void> {
    if (retrying) return;
    retrying = true;
    try {
      await load();
    } finally {
      retrying = false;
    }
    if (!loadFailed) {
      await tick();
      form?.querySelector('input')?.focus();
    }
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
      const result = await saved.track(api.set(defaultsInput(draft)), (outcome) => outcome.ok);
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

<!-- A group, not a form landmark; Enter still submits. The Settings page says what the section is for (SET-001). -->
<form bind:this={form} role="group" aria-label="Default check settings" novalidate onsubmit={submit}>
  {#each SETTING_FIELDS as { key, label, bounds } (key)}
    <SettingRow {label} for="{id}-{key}">
      {#snippet hint()}
        <p class="hint" id="{id}-{key}-hint">Allowed {bounds[0]} to {bounds[1]}.</p>
      {/snippet}
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
      {#snippet message()}
        <!-- The message keeps its line whether or not it has text, so nothing below moves (MOT-007, MV-005). -->
        <div class="problem" id="{id}-{key}-problem" role="alert">{messageFor(errors, key) ?? ''}</div>
      {/snippet}
    </SettingRow>
  {/each}
  <!-- The footer holds the section's notice and Save on one line in every state. The notice's icon and Try again stay
       mounted and are hidden unless needed, so the footer keeps its height and focus stays put (MOT-007, MV-005, UI-015). -->
  <div class="footer">
    <div class="note-block">
      <span class="mark" class:hidden={notice === ''}><Icon name="carbon:warning" size="sm" /></span>
      <p class="message" role="alert">{notice}</p>
      <div class="retry" class:hidden={!loadFailed}>
        <FormButton tabindex={0} data-retry aria-disabled={retrying} onclick={retry}>Try again</FormButton>
      </div>
    </div>
    <FormButton tabindex={0} type="submit" tone="primary" aria-label="Save default check settings" aria-disabled={loaded === null || busy} feedback={saved.state}>Save</FormButton>
  </div>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
  }

  .footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: end;
    gap: var(--space-2) var(--space-4);
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-strong);
  }

  .note-block {
    display: flex;
    flex: 1 1 0;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  .mark {
    display: inline-flex;
    color: var(--status-down);
  }

  /* The notice text stays --text (MAT-005); the icon beside it marks the error, never colour alone (UI-003). */
  .message {
    flex: 1 1 0;
    min-inline-size: 0;
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .hidden {
    visibility: hidden;
  }
</style>
