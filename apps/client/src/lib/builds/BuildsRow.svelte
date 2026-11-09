<script lang="ts">
  import { desktopBuilderShell } from '$lib/desktop/builder/shell';
  import { importUpdaterKey, readUpdaterKey, unlockUpdaterKey } from '$lib/desktop/builder/updaterKey';
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
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

<section aria-label="Builds">
  <label class="toggle">
    <input
      type="checkbox"
      checked={builder.enrolled === true}
      aria-disabled={toggleBlocked}
      aria-describedby={builder.problem === null ? undefined : 'builder-problem'}
      onclick={blockWhileBusy}
      onchange={(event) => void toggle(event)}
    />
    <span>Build releases on this computer</span>
  </label>
  <!-- Each line keeps its height while it is empty, so the controls below do not move when it fills (MOT-007). -->
  <p class="line">{thisPlatformText(panel.tools, panel.toolsFailed)}</p>
  <p class="line muted" aria-live="polite">{builderText(builder.enrolled, builder.busy, builder.status)}</p>
  {#if missing.length > 0}
    <p id="missing-tools">Missing:</p>
    <ul class="missing" aria-labelledby="missing-tools">
      {#each missing as tool (tool)}
        <li>{TOOL_NAMES[tool]}</li>
      {/each}
    </ul>
  {/if}
  {#if builder.problem !== null}
    <p id="builder-problem" role="alert">{builder.problem}</p>
  {/if}

  <p class="line">{keyText(panel.key)}</p>
  {#if memoryText(panel.tools) !== ''}
    <p class="warning">{memoryText(panel.tools)}</p>
  {/if}
  {#if panel.key?.publicKey}
    <p class="line muted key">Public key <code>{panel.key.publicKey}</code></p>
  {/if}
  {#if locked}
    <div class="fields">
      <label for="updater-key-unlock">Updater key password</label>
      <input
        id="updater-key-unlock"
        type="password"
        autocomplete="off"
        aria-invalid={panel.keyField === 'unlock-password'}
        aria-describedby={described('unlock-password')}
        bind:this={fields['unlock-password']}
        bind:value={unlockPassword}
      />
      <button type="button" tabindex="0" aria-disabled={keyBusy} onclick={() => void unlockKey()} {@attach press()}>
        Unlock updater key
      </button>
    </div>
  {/if}
  <div class="fields">
    <label for="updater-key-password">Key file password</label>
    <input
      id="updater-key-password"
      type="password"
      autocomplete="off"
      aria-invalid={panel.keyField === 'file-password'}
      aria-describedby={described('file-password')}
      bind:this={fields['file-password']}
      bind:value={password}
    />
    {#if panel.newPasswordNeeded}
      <label for="updater-key-new-password">New password for the key</label>
      <input
        id="updater-key-new-password"
        type="password"
        autocomplete="new-password"
        aria-invalid={panel.keyField === 'new-password'}
        aria-describedby={described('new-password')}
        bind:this={fields['new-password']}
        bind:value={newPassword}
      />
    {/if}
    <button type="button" tabindex="0" aria-disabled={keyBusy} onclick={() => void importKey()} {@attach press()}>
      Import updater key
    </button>
  </div>
  {#if panel.keyProblem !== null}
    <p id="updater-key-problem" role="alert">{panel.keyProblem}</p>
  {/if}

  <ul class="builders" aria-label="Builders" aria-describedby={panel.listProblem === null ? undefined : 'builders-problem'}>
    {#each panel.builders as each (each.id)}
      <li>
        <span class="name">{each.name}</span>
        <span class="muted">{platformText(each.platform)}, {each.online ? 'online' : 'offline'}</span>
        <button
          type="button"
          tabindex="0"
          aria-disabled={panel.revoking !== null}
          aria-label={`Revoke ${each.name}`}
          onclick={() => void panel.revoke(each.id)}
          {@attach press()}>Revoke</button
        >
      </li>
    {:else}
      <!-- Until the list is read, an empty line holds its place rather than saying no computer builds (MOT-007). -->
      <li class={panel.buildersLoaded ? 'muted' : 'line'}>{panel.buildersLoaded ? 'No computer builds releases.' : ''}</li>
    {/each}
  </ul>
  {#if panel.listProblem !== null}
    <p id="builders-problem" role="alert">{panel.listProblem}</p>
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

  p,
  label,
  ul {
    margin: 0;
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  ul {
    padding: 0;
    list-style: none;
  }

  .line {
    min-block-size: calc(var(--text-md) * var(--leading-tight));
  }

  .muted {
    color: var(--text-muted);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--space-8);
  }

  .missing {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding-inline-start: var(--space-4);
    list-style: disc;
  }

  .key {
    max-inline-size: 100%;
    overflow-wrap: anywhere;
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .builders {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-block-start: var(--space-4);
  }

  .builders li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .name {
    font-weight: var(--weight-bold);
  }

  input[type='password'],
  button {
    box-sizing: border-box;
    max-inline-size: 100%;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-lg);
  }

  button {
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not([aria-disabled='true']) {
    background: var(--surface-raised);
  }

  button[aria-disabled='true'] {
    color: var(--text-muted);
    cursor: default;
  }

  input:focus-visible,
  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .warning {
    max-inline-size: 100%;
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-warning);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
