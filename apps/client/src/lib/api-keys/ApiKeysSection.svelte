<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import { useHub } from '$lib/hub/context';
  import type { HubResult } from '$lib/hub/hub';
  import { apiKeysApi, type ApiKey } from './api';
  import KeyRow from './KeyRow.svelte';
  import NewKeyDialog from './NewKeyDialog.svelte';

  interface Props {
    /** The clock for expiry marks and new keys' expiry dates (CQ-021). */
    now?: () => Date;
  }

  const { now = () => new Date() }: Props = $props();
  const api = apiKeysApi(useHub());
  const headingId = $props.id();
  /** `null` while loading or after a failed load. */
  let keys = $state<readonly ApiKey[] | null>(null);
  let loadFailed = $state(false);
  let creating = $state(false);
  let revoking = $state<ApiKey | null>(null);

  async function load(): Promise<void> {
    loadFailed = false;
    const result = await api.list();
    if (result.ok) keys = result.value;
    else loadFailed = true;
  }

  $effect(() => {
    void load();
  });

  function created(key: ApiKey): void {
    if (keys !== null) keys = [...keys, key];
  }

  /** The confirmed revoke; a new modal for the next one, since a finished one keeps its buttons off. */
  async function revoke(): Promise<HubResult<unknown>> {
    if (revoking === null) return { ok: true, value: undefined };
    const { id } = revoking;
    const result = await api.revoke(id);
    if (result.ok) {
      keys = keys?.filter((key) => key.id !== id) ?? null;
      revoking = null;
    }
    return result;
  }
</script>

<section aria-labelledby={headingId}>
  <div class="head">
    <h2 id={headingId}>API keys</h2>
    <FormButton tabindex={0} onclick={() => { creating = true; }}>Create API key</FormButton>
  </div>
  {#if loadFailed}
    <div class="failed">
      <p role="alert">Anima could not load the API keys.</p>
      <FormButton tabindex={0} onclick={load}>Try again</FormButton>
    </div>
  {:else if keys === null}
    <p class="quiet" role="status">Loading API keys…</p>
  {:else if keys.length === 0}
    <p class="quiet">No API keys. Programs that call the hub's GraphQL API need one.</p>
  {:else}
    <ul>
      {#each keys as key (key.id)}
        <KeyRow {key} expiredAt={now()} onRevoke={() => { revoking = key; }} />
      {/each}
    </ul>
  {/if}
</section>

{#if creating}
  <NewKeyDialog {api} {now} onCreated={created} onClose={() => { creating = false; }} />
{/if}

{#key revoking?.id}
  <ConfirmModal
    open={revoking !== null}
    title="Revoke {revoking?.name ?? ''}?"
    body="Programs that use it stop working at once."
    confirmLabel="Revoke {revoking?.name ?? ''}"
    onConfirm={revoke}
    onCancel={() => {
      revoking = null;
    }}
  />
{/key}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-block-start: var(--space-6);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    line-height: var(--leading-tight);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
  }

  .quiet {
    color: var(--text-muted);
  }

  .failed {
    display: flex;
    flex-direction: column;
    align-items: start;
    gap: var(--space-2);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
