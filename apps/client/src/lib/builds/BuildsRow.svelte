<script lang="ts">
  import { desktopBuilderShell } from '$lib/desktop/builder/shell';
  import { importUpdaterKey, readUpdaterKey, unlockUpdaterKey } from '$lib/desktop/builder/updater-key';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { buildsApi } from './api';
  import { useThisBuilder } from './builder.svelte';
  import { BuildsPanel, type KeyField } from './panel.svelte';
  import { builderText, keyText, memoryText, missingTools, platformText, thisPlatformText, TOOL_NAMES } from './row';

  const builder = useThisBuilder();
  const panel = new BuildsPanel({
    api: buildsApi(useHub()),
    shell: desktopBuilderShell,
    updaterKey: { read: readUpdaterKey, import: importUpdaterKey, unlock: unlockUpdaterKey },
    revoked: (id) => builder.revoked(id),
  });
  // The passwords live here only until the shell has used them; they are never stored or logged (LOG-007).
  let password = $state('');
  let newPassword = $state('');
  let unlockPassword = $state('');
  const fields: Partial<Record<KeyField, HTMLInputElement>> = {};

  const missing = $derived(missingTools(builder.status.kind === 'missing' ? builder.status.tools : (panel.tools?.missing ?? [])));
  const toggleBlocked = $derived(builder.busy || builder.enrolled === null);
  const keyBusy = $derived(panel.importing || panel.unlocking);
  const locked = $derived(panel.key?.present === true && !panel.key.unlocked);

  $effect(() => {
    void panel.load();
  });

  // The field a key problem is about takes the focus, so the operator can type at once (UI-013).
  $effect(() => {
    if (panel.keyField !== null) fields[panel.keyField]?.focus();
  });

  /** While busy the toggle stays focusable but ignores activation (UI-013). */
  function blockWhileBusy(event: MouseEvent): void {
    if (toggleBlocked) event.preventDefault();
  }

  async function toggle(event: Event & { currentTarget: HTMLInputElement }): Promise<void> {
    if (event.currentTarget.checked) await builder.enable();
    else await builder.disable();
    await Promise.all([panel.loadBuilders(), panel.loadKey()]);
  }

  async function importKey(): Promise<void> {
    if (keyBusy) return;
    if (await panel.importKey(password, newPassword)) {
      password = '';
      newPassword = '';
    }
  }

  async function unlockKey(): Promise<void> {
    if (keyBusy) return;
    if (await panel.unlockKey(unlockPassword)) unlockPassword = '';
  }

  function described(field: KeyField): string | undefined {
    return panel.keyField === field ? 'updater-key-problem' : undefined;
  }
</script>

{#snippet importButton()}
  <FormButton tabindex={0} aria-disabled={keyBusy} onclick={() => void importKey()}>Import updater key</FormButton>
{/snippet}

<!-- Rows of the Updates card, like the version and Logs rows above: name and hint at the start, control at the end (SET-001). -->
<SettingRow label="Build releases on this computer" for="builder-toggle">
  {#snippet hint()}
    <!-- Each line keeps its height while it is empty, so the controls below do not move when it fills (MOT-007). -->
    <p class="hint">{thisPlatformText(panel.tools, panel.toolsFailed)}</p>
    <p class="hint" aria-live="polite">{builderText(builder.enrolled, builder.busy, builder.status)}</p>
    {#if missing.length > 0}
      <p class="hint" id="missing-tools">Missing:</p>
      <ul class="hint missing" aria-labelledby="missing-tools">
        {#each missing as tool (tool)}
          <li>{TOOL_NAMES[tool]}</li>
        {/each}
      </ul>
    {/if}
  {/snippet}
  <input
    id="builder-toggle"
    class="toggle"
    type="checkbox"
    checked={builder.enrolled === true}
    aria-disabled={toggleBlocked}
    aria-describedby={builder.problem === null ? undefined : 'builder-problem'}
    onclick={blockWhileBusy}
    onchange={(event) => void toggle(event)}
  />
  {#snippet message()}
    <p id="builder-problem" role="alert">{builder.problem ?? ''}</p>
  {/snippet}
</SettingRow>

{#if locked}
  <SettingRow label="Updater key password" for="updater-key-unlock">
    <input
      id="updater-key-unlock"
      class="secret"
      type="password"
      autocomplete="off"
      aria-invalid={panel.keyField === 'unlock-password'}
      aria-describedby={described('unlock-password')}
      bind:this={fields['unlock-password']}
      bind:value={unlockPassword}
    />
    <FormButton tabindex={0} aria-disabled={keyBusy} onclick={() => void unlockKey()}>Unlock updater key</FormButton>
  </SettingRow>
{/if}

<SettingRow label="Key file password" for="updater-key-password">
  {#snippet hint()}
    <p class="hint">{keyText(panel.key)}</p>
    {#if memoryText(panel.tools) !== ''}
      <p class="hint warning">{memoryText(panel.tools)}</p>
    {/if}
    {#if panel.key?.publicKey}
      <p class="hint key">Public key <code>{panel.key.publicKey}</code></p>
    {/if}
  {/snippet}
  <input
    id="updater-key-password"
    class="secret"
    type="password"
    autocomplete="off"
    aria-invalid={panel.keyField === 'file-password'}
    aria-describedby={described('file-password')}
    bind:this={fields['file-password']}
    bind:value={password}
  />
  {#if !panel.newPasswordNeeded}{@render importButton()}{/if}
  {#snippet message()}
    {#if !panel.newPasswordNeeded}<p id="updater-key-problem" role="alert">{panel.keyProblem ?? ''}</p>{/if}
  {/snippet}
</SettingRow>
{#if panel.newPasswordNeeded}
  <SettingRow label="New password for the key" for="updater-key-new-password">
    <input
      id="updater-key-new-password"
      class="secret"
      type="password"
      autocomplete="new-password"
      aria-invalid={panel.keyField === 'new-password'}
      aria-describedby={described('new-password')}
      bind:this={fields['new-password']}
      bind:value={newPassword}
    />
    {@render importButton()}
    {#snippet message()}
      <p id="updater-key-problem" role="alert">{panel.keyProblem ?? ''}</p>
    {/snippet}
  </SettingRow>
{/if}

<ul class="builders" aria-label="Builders" aria-describedby={panel.listProblem === null ? undefined : 'builders-problem'}>
  {#each panel.builders as each (each.id)}
    <li>
      <SettingRow label={each.name}>
        {#snippet hint()}
          <p class="hint">{platformText(each.platform)}, {each.online ? 'online' : 'offline'}</p>
        {/snippet}
        <FormButton tabindex={0} tone="danger" aria-disabled={panel.revoking !== null} aria-label={`Revoke ${each.name}`} onclick={() => void panel.revoke(each.id)}>Revoke</FormButton>
      </SettingRow>
    </li>
  {:else}
    <!-- Until the list is read, an empty line holds its place rather than saying no computer builds (MOT-007). -->
    <li class="note">{panel.buildersLoaded ? 'No computer builds releases.' : ''}</li>
  {/each}
</ul>
{#if panel.listProblem !== null}
  <p class="note" id="builders-problem" role="alert">{panel.listProblem}</p>
{/if}

<style>
  /* The one line a row of the card keeps for a list that may be empty (MOT-007). */
  .note {
    box-sizing: border-box;
    min-block-size: calc(var(--text-sm) * var(--leading-tight) + 2 * var(--space-3) + var(--border-thin));
    margin: 0;
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-strong);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .builders {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .missing {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding-inline-start: var(--space-4);
    list-style: disc;
  }

  .key {
    overflow-wrap: anywhere;
  }

  .warning {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-warning);
    color: var(--text);
  }

  /* A password is wider than a number, which the row's inputs are sized for. */
  input[type='password'].secret {
    inline-size: calc(var(--space-8) * 6);
    max-inline-size: 100%;
    text-align: start;
  }

  /* The row's input look is for text fields; a checkbox keeps its own. */
  input[type='checkbox'].toggle {
    inline-size: var(--space-5);
    block-size: var(--space-5);
    padding: 0;
    accent-color: var(--accent);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  .note[role='alert'] {
    padding-inline-start: var(--space-4);
    border-inline-start: var(--border-focus) solid var(--status-down);
    color: var(--text);
  }
</style>
