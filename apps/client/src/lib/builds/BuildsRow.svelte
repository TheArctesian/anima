<script lang="ts">
  import { desktopBuilderShell } from '$lib/desktop/builder/shell';
  import { importUpdaterKey, readUpdaterKey } from '$lib/desktop/builder/updaterKey';
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
  import { buildsApi } from './api';
  import { useThisBuilder } from './builder.svelte';
  import { BuildsPanel } from './panel.svelte';
  import { keyText, platformText, statusText, thisPlatformText, TOOL_NAMES } from './row';

  const builder = useThisBuilder();
  const panel = new BuildsPanel({
    api: buildsApi(useHub()),
    shell: desktopBuilderShell,
    updaterKey: { read: readUpdaterKey, import: importUpdaterKey },
    revoked: (id) => builder.revoked(id),
  });
  // The key password lives here only until the import has used it; it is never stored or logged (LOG-007).
  let password = $state('');

  const missing = $derived(builder.status.kind === 'missing' ? builder.status.tools : (panel.tools?.missing ?? []));

  $effect(() => {
    void panel.load();
  });

  async function toggle(event: Event & { currentTarget: HTMLInputElement }): Promise<void> {
    if (event.currentTarget.checked) await builder.enable();
    else await builder.disable();
    await panel.loadBuilders();
  }

  async function importKey(): Promise<void> {
    if (await panel.importKey(password)) password = '';
  }
</script>

<section aria-label="Builds">
  <label class="toggle">
    <input
      type="checkbox"
      checked={builder.enrolled === true}
      disabled={builder.busy || builder.enrolled === null}
      onchange={(event) => void toggle(event)}
    />
    <span>Build releases on this computer</span>
  </label>
  <!-- Each line keeps its height while it is empty, so the controls below do not move when it fills (MOT-007). -->
  <p class="line">{thisPlatformText(panel.tools, panel.toolsFailed)}</p>
  <p class="line muted" aria-live="polite">{builder.enrolled === true ? statusText(builder.status) : 'Off.'}</p>
  {#if missing.length > 0}
    <ul class="missing" aria-label="Missing tools">
      {#each missing as tool (tool)}
        <li>{TOOL_NAMES[tool]} is missing.</li>
      {/each}
    </ul>
  {/if}
  {#if builder.problem !== null}
    <p role="alert">{builder.problem}</p>
  {/if}

  <p class="line">{keyText(panel.key)}</p>
  {#if panel.key?.publicKey}
    <p class="line muted key">Public key <code>{panel.key.publicKey}</code></p>
  {/if}
  <div class="import">
    <label for="updater-key-password">Key file password</label>
    <input id="updater-key-password" type="password" autocomplete="off" bind:value={password} />
    <button type="button" tabindex="0" disabled={panel.importing} onclick={() => void importKey()} {@attach press()}>
      Import updater key
    </button>
  </div>
  {#if panel.keyProblem !== null}
    <p role="alert">{panel.keyProblem}</p>
  {/if}

  <ul class="builders" aria-label="Builders">
    {#each panel.builders as each (each.id)}
      <li>
        <span class="name">{each.name}</span>
        <span class="muted">{platformText(each.platform)}, {each.online ? 'online' : 'offline'}</span>
        <button
          type="button"
          tabindex="0"
          disabled={panel.revoking !== null}
          aria-label={`Revoke ${each.name}`}
          onclick={() => void panel.revoke(each.id)}
          {@attach press()}>Revoke</button
        >
      </li>
    {:else}
      <li class="muted">No computer builds releases.</li>
    {/each}
  </ul>
  {#if panel.listProblem !== null}
    <p role="alert">{panel.listProblem}</p>
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
  }

  .key {
    max-inline-size: 100%;
    overflow-wrap: anywhere;
  }

  .import {
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
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  button {
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:enabled {
    background: var(--surface-raised);
  }

  button:disabled {
    color: var(--text-muted);
    cursor: default;
  }

  input:focus-visible,
  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
