<script lang="ts">
  import type { HubResult } from '$lib/hub/hub';
  import FormButton from '../../../forms/FormButton.svelte';
  import type { WebInterface, WebInterfaceInput, WebInterfaceSuggestion } from '$lib/web-interfaces/api';
  import ConfirmModal from '../../../forms/modal/ConfirmModal.svelte';
  import Suggestions from './Suggestions.svelte';
  import WebInterfaceRow from './WebInterfaceRow.svelte';

  /** The hub calls the section makes; the owner refreshes the lists after each one (D-6). */
  export interface WebInterfaceCalls {
    onCreate: (input: WebInterfaceInput) => Promise<HubResult<unknown>>;
    onUpdate: (id: string, input: WebInterfaceInput) => Promise<HubResult<unknown>>;
    onDelete: (id: string) => Promise<HubResult<unknown>>;
    onAcceptSuggestion: (checkId: string) => Promise<HubResult<unknown>>;
    onDismissSuggestion: (checkId: string) => Promise<HubResult<unknown>>;
  }

  interface Props {
    interfaces: readonly WebInterface[];
    suggestions: readonly WebInterfaceSuggestion[];
    calls: WebInterfaceCalls;
  }

  const { interfaces, suggestions, calls }: Props = $props();
  let adding = $state(false);
  let deleting = $state<WebInterface | null>(null);

  async function create(input: WebInterfaceInput): Promise<HubResult<unknown>> {
    const result = await calls.onCreate(input);
    if (result.ok) adding = false;
    return result;
  }

  /** The confirmed delete; a new modal for the next one, since a finished one keeps its buttons off. */
  async function remove(): Promise<HubResult<unknown>> {
    if (deleting === null) return { ok: true, value: undefined };
    const result = await calls.onDelete(deleting.id);
    if (result.ok) deleting = null;
    return result;
  }
</script>

<section aria-labelledby="web-interfaces-heading">
  <h3 id="web-interfaces-heading">Web interfaces</h3>
  <Suggestions suggestions={suggestions} onAccept={calls.onAcceptSuggestion} onDismiss={calls.onDismissSuggestion} />
  {#if interfaces.length === 0 && !adding}
    <p class="empty">This device has no web interface.</p>
  {/if}
  <ul>
    {#each interfaces as web (web.id)}
      <li aria-label={web.label}>
        <!-- Remounted when the hub's stored values change, so the row shows what the hub stored (for example the host as an empty label). -->
        {#key [web.label, web.url, web.openIn].join('\n')}
        <WebInterfaceRow
          saved={web}
          rowKey="web-{web.id}"
          onSave={(input) => calls.onUpdate(web.id, input)}
          onDelete={() => {
            deleting = web;
          }}
        />
        {/key}
      </li>
    {/each}
  </ul>
  {#if adding}
    <WebInterfaceRow rowKey="web-new" onSave={create} onCancel={() => { adding = false; }} />
  {:else}
    <div><FormButton onclick={() => { adding = true; }}>Add a web interface</FormButton></div>
  {/if}
</section>

{#key deleting?.id}
  <ConfirmModal
    open={deleting !== null}
    title="Delete {deleting?.label ?? ''}?"
    body="Anima stops showing {deleting?.url ?? ''} for this device."
    confirmLabel="Delete {deleting?.label ?? ''}"
    onConfirm={remove}
    onCancel={() => {
      deleting = null;
    }}
  />
{/key}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-block-start: var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  h3 {
    margin: 0;
    font-size: var(--text-md);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
  }
</style>
