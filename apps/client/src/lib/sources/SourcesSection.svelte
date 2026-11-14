<script lang="ts">
  import { untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import AccountRow from './accounts/AccountRow.svelte';
  import DisconnectDialog from './accounts/DisconnectDialog.svelte';
  import { sourcesApi } from './api';
  import ConnectGithubDialog from './connect/ConnectGithubDialog.svelte';
  import { SourcesStore, type AccountRow as Account } from './store.svelte';

  // The Settings page draws the section and its heading; these are its rows (ITG-001). The operator makes the GitHub App
  // in their own account; Anima only keeps its key (PRJ-018).
  interface Props {
    /** The clock for "Synced 4 min ago"; tests pass their own. */
    now?: () => Date;
  }

  const { now = () => new Date() }: Props = $props();
  // The hub is fixed for the life of the section.
  const store = untrack(() => new SourcesStore(sourcesApi(useHub())));
  let connecting = $state(false);
  let disconnecting = $state<Account | null>(null);

  $effect(() => {
    void store.ensureLoaded();
    return () => {
      store.stop();
    };
  });

  function disconnect(deleteRecords: boolean): void {
    const account = disconnecting;
    disconnecting = null;
    if (account !== null) void store.remove(account.id, deleteRecords);
  }
</script>

<SettingRow label="GitHub">
  {#snippet hint()}
    <p class="hint">Reads your repositories through a GitHub App that you make in your own account. Anima does not make it.</p>
  {/snippet}
  <FormButton
    icon="simple-icons:github"
    onclick={() => {
      connecting = true;
    }}
  >
    Connect GitHub
  </FormButton>
</SettingRow>

{#if store.status === 'failed'}
  <ErrorState message={store.loadProblem} onRetry={() => void store.load()} />
{:else}
  <ul class="accounts" aria-busy={store.status === 'loading'}>
    {#each store.rows as account (account.id)}
      <li>
        <AccountRow
          {account}
          now={now()}
          refusal={store.refusals[account.id] ?? ''}
          working={store.working[account.id] === true}
          onSync={() => void store.sync(account.id)}
          onPause={(paused) => void store.setPaused(account.id, paused)}
          onDisconnect={() => {
            disconnecting = account;
          }}
        />
      </li>
    {/each}
  </ul>
  {#if store.status === 'ready' && store.rows.length === 0}
    <p class="hint none">No source is connected.</p>
  {/if}
{/if}

{#if connecting}
  <ConnectGithubDialog
    onConnect={(input) => store.connect(input)}
    onClose={() => {
      connecting = false;
    }}
  />
{/if}

{#if disconnecting !== null}
  <DisconnectDialog
    label={disconnecting.label}
    sourceName={disconnecting.sourceName}
    onConfirm={disconnect}
    onCancel={() => {
      disconnecting = null;
    }}
  />
{/if}

<style>
  .accounts {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .none {
    margin: 0;
    padding-block: var(--space-3);
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }
</style>
