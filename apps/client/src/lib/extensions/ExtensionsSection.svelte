<script lang="ts">
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
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

  function change(event: Event & { currentTarget: HTMLInputElement }, key: string, label: string): void {
    const input = event.currentTarget;
    const on = input.checked;
    const sentence = on ? alsoSetsSentence(key, store.settled) : null;
    if (sentence === null) {
      void store.set(key, on);
      return;
    }
    // The switch stays where it was until the operator says yes.
    input.checked = !on;
    confirming = { key, name: label, sentence };
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
            <input
              id="extension-{row.key}"
              class="toggle"
              type="checkbox"
              role="switch"
              checked={row.on}
              disabled={row.kind === 'needed'}
              aria-describedby="extension-{row.key}-hint"
              onchange={(event) => {
                change(event, row.key, row.label);
              }}
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

  /* The row's input look is for text fields; a checkbox keeps its own. */
  input[type='checkbox'].toggle {
    inline-size: var(--space-5);
    block-size: var(--space-5);
    padding: 0;
    accent-color: var(--accent);
  }
</style>
