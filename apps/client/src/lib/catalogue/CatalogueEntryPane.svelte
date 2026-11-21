<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { CatalogueEntry } from './api';
  import { SHELL_REASON, TIER_LABELS, UNREVIEWED_NOTE, enumWords, markWords, partState, protocolWords, reviewFact, tierFact, unreviewed } from './entry';

  /**
   * Tasks H38 and H39: the description of one entry. The install button is present and refused with the hub's fixed
   * reason until the install exists (task H6 of the hosted applications plan). The tier, the class, the streams and the
   * optional parts say what the entry needs beyond an application that Anima tested. Nothing here reaches the device.
   */
  const { entry, onclose }: { entry: CatalogueEntry; onclose: () => void } = $props();
</script>

<section class="pane" aria-label="About {entry.name}">
  <header class="bar">
    <h3>{entry.name}</h3>
    <span class="mark">{TIER_LABELS[entry.tier]}</span>
    {#if entry.class === 'SHELL'}<span class="mark">Shell</span>{/if}
    {#if unreviewed(entry)}<span class="mark">Not reviewed</span>{/if}
    <FormButton size="sm" tone="primary" aria-disabled={entry.install.refusal !== null} aria-describedby="install-reason">Install on this device</FormButton>
    <FormButton size="sm" tone="ghost" icon="carbon:close" label="Close the description" onclick={onclose} />
  </header>
  <p class="reason" id="install-reason" role="note"><Icon name="carbon:information" size="sm" />{entry.install.reason ?? 'The hub can install this entry.'}</p>
  {#if unreviewed(entry)}
    <p class="reason" role="note"><Icon name="carbon:warning-alt" size="sm" />{UNREVIEWED_NOTE}</p>
  {/if}
  {#if entry.install.needsOpenToNetwork}
    <p class="reason">It needs the setting Open to the network at the machine.</p>
  {/if}
  <p class="description">{entry.description}</p>
  <dl class="facts">
    <dt>Tier</dt><dd>{tierFact(entry)}</dd>
    {#if reviewFact(entry) !== null}<dt>Reviewed</dt><dd>{reviewFact(entry)}</dd>{/if}
    {#if entry.class === 'SHELL'}<dt>Class</dt><dd>Shell. {SHELL_REASON}</dd>{/if}
    {#if entry.kind !== 'APPLICATION'}<dt>Kind</dt><dd>{enumWords(entry.kind)}</dd>{/if}
    <dt>Version</dt><dd class="mono">{entry.version}</dd>
    <dt>Author</dt><dd>{entry.author}</dd>
    <dt>Tags</dt><dd>{entry.tags.join(', ')}</dd>
    <dt>Streams</dt>
    <dd>
      {#each entry.streams as stream (stream.port)}
        <span class="part"><span class="mono">{stream.port}</span>{protocolWords(stream.protocols)}</span>
      {:else}<span class="none">None</span>{/each}
    </dd>
    <dt>Optional parts</dt>
    <dd>
      {#each entry.optionalParts as part (part.path)}
        <span class="part">
          <span class="mono">{part.path}</span>
          <label class="state"><Switch checked={part.on} disabled label="{part.path}: {partState(part)}" /><span aria-hidden="true">{partState(part)}</span></label>
          {#if part.needsMachineSetting}<span class="mark">Needs machine setting</span>{/if}
        </span>
      {:else}<span class="none">None</span>{/each}
    </dd>
    <dt>Images pinned</dt><dd>{entry.pinned}</dd>
    <dt>Marks</dt><dd>{markWords(entry)}</dd>
    <dt>Variables</dt><dd class="mono">{entry.environmentVariables.join(', ')}</dd>
    <dt>Documentation</dt>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an https:// address of the recipe, outside the application -->
    <dd><a href={entry.documentationUrl} target="_blank" rel="noopener noreferrer">{entry.documentationUrl}</a></dd>
  </dl>
  <h4>Compose file</h4>
  <pre class="file">{entry.composeFile}</pre>
</section>

<style>
  /* The description is one raised card beside the list, as the log of a container is under its table (R-224). */
  .pane {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-block-size: 0;
    min-inline-size: 0;
    padding: var(--space-4);
    overflow: auto;
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  h3 {
    flex: 1;
    margin: 0;
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  h4 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .reason {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
  }

  .description {
    color: var(--text);
  }

  .facts {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--space-1) var(--space-4);
    margin: 0;
    font-size: var(--text-sm);
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    overflow-wrap: anywhere;
    color: var(--text);
  }

  .part,
  .state {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .state {
    color: var(--text-secondary);
  }

  .none {
    color: var(--text-secondary);
  }

  /* A neutral mark: the tier, the class and a needed setting are not a status (5.1.2). */
  .mark {
    flex: none;
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  .mono,
  .file {
    font-family: var(--font-mono);
  }

  .file {
    margin: 0;
    padding: var(--space-3);
    overflow: auto;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text);
    font-size: var(--text-sm);
  }
</style>
