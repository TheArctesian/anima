<script lang="ts">
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { alsoSetsSentence, extensionRows } from './switches';
  import { useExtensions } from './store.svelte';

  // The Settings page draws the section and its heading; these are its rows. A change is saved at once (EXT-001).
  const store = useExtensions();
  const rows = $derived(extensionRows(store.shown));
  /** The switch-on that waits for the operator's yes, with the sentence of EXT-011. */
  let confirming = $state<{ key: string; name: string; sentence: string } | null>(null);

  $effect(() => {
    void store.ensureLoaded();
  });

  /** False keeps the switch where it was until the operator says yes. */
  function change(on: boolean, key: string, label: string): boolean {
    const sentence = on ? alsoSetsSentence(key, store.settled) : null;
    if (sentence === null) {
      void store.set(key, on);
      return true;
    }
    confirming = { key, name: label, sentence };
    return false;
  }

  async function confirmed(): Promise<{ ok: true; value: undefined }> {
    const waiting = confirming;
    confirming = null;
    if (waiting !== null) void store.set(waiting.key, true);
    return { ok: true, value: undefined };
  }
</script>

{#if store.status === 'failed'}
  <ErrorState message={store.loadProblem} onRetry={() => void store.load()} />
{:else}
  <ul class="rows" aria-busy={store.status === 'loading'}>
    {#each rows as row (row.key)}
      <li>
        <SettingRow label={row.label} for={row.kind === 'switch' || row.kind === 'needed' ? `extension-${row.key}` : undefined}>
          {#snippet hint()}
            <p class="hint" id="extension-{row.key}-hint">{row.hint}</p>
          {/snippet}
          {#if row.kind === 'switch' || row.kind === 'needed'}
            <Switch
              id="extension-{row.key}"
              checked={row.on}
              disabled={row.kind === 'needed'}
              describedby="extension-{row.key}-hint"
              onchange={(on) => change(on, row.key, row.label)}
            />
          {:else if row.kind === 'fixed'}
            <span class="state">On</span>
          {/if}
          {#snippet message()}
            <p role="alert">{store.refusal?.key === row.key ? store.refusal.message : ''}</p>
          {/snippet}
        </SettingRow>
      </li>
    {/each}
  </ul>
{/if}

{#key confirming}
  <ConfirmModal
    open={confirming !== null}
    title={confirming === null ? '' : `Set ${confirming.name} on`}
    body={confirming?.sentence ?? ''}
    confirmLabel="Set on"
    onConfirm={confirmed}
    onCancel={() => {
      confirming = null;
    }}
  />
{/key}

<style>
  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .state {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
