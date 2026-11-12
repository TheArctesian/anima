<script lang="ts">
  import { desktopBuilderShell } from '$lib/desktop/builder/shell';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { buildsApi } from './api';
  import { useThisBuilder } from './builder.svelte';
  import { BuildsPanel } from './panel.svelte';
  import { builderText, platformText, thisPlatformText, TOOL_NAMES } from './row';

  const builder = useThisBuilder();
  const panel = new BuildsPanel({
    api: buildsApi(useHub()),
    shell: desktopBuilderShell,
    revoked: (id) => builder.revoked(id),
  });

  const missing = $derived(builder.status.kind === 'missing' ? builder.status.tools : (panel.tools?.missing ?? []));
  const toggleBlocked = $derived(builder.busy || builder.enrolled === null);

  $effect(() => {
    void panel.load();
  });

  /** While busy the toggle stays focusable but ignores activation (UI-013). */
  function blockWhileBusy(event: MouseEvent): void {
    if (toggleBlocked) event.preventDefault();
  }

  async function toggle(event: Event & { currentTarget: HTMLInputElement }): Promise<void> {
    if (event.currentTarget.checked) await builder.enable();
    else await builder.disable();
    await panel.loadBuilders();
  }
</script>

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
