<script lang="ts">
  import { tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { crossFade } from '$lib/motion/roles/transitions';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { apiKeysApi, type ApiKey } from './api';
  import KeyRow from './KeyRow.svelte';
  import { focusAfterRevoke, PLACEHOLDER_KEY } from './listing';
  import NewKeyDialog from './NewKeyDialog.svelte';

  // The Settings page draws the section and its heading (F-10); this is what goes under the heading.
  interface Props {
    /** The clock for expiry marks and new keys' expiry dates (CQ-021). */
    now?: () => Date;
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { now = () => new Date(), clock }: Props = $props();
  const api = apiKeysApi(useHub());
  /** `null` while loading or after a failed load. */
  let keys = $state<readonly ApiKey[] | null>(null);
  let loadFailed = $state(false);
  let retrying = $state(false);
  let creating = $state(false);
  let revoking = $state<ApiKey | null>(null);
  let root: HTMLElement | undefined = $state();
  const loading = $derived(keys === null && !loadFailed);
  // The clock is fixed for the life of the section.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);

  async function load(): Promise<void> {
    const result = await api.list();
    if (result.ok) {
      keys = result.value;
      loadFailed = false;
    } else loadFailed = true;
  }

  /** Try again: the button stays mounted, and aria-disabled, while the hub answers; on success focus goes to Create (NAV-005). */
  async function retry(): Promise<void> {
    if (retrying) return;
    retrying = true;
    try {
      await load();
    } finally {
      retrying = false;
    }
    if (!loadFailed) {
      await tick();
      root?.querySelector<HTMLElement>('[data-create]')?.focus();
    }
  }

  $effect(() => {
    void load();
  });

  function created(key: ApiKey): void {
    if (keys !== null) keys = [...keys, key];
  }

  /** Focus for the operator after a revoke: the row that took its place, the row before, or Create (NAV-005). */
  function focusRow(id: string | null): void {
    const revokes = [...(root?.querySelectorAll<HTMLElement>('.list [data-revoke]') ?? [])];
    const target = revokes.find((button) => button.dataset.revoke === id) ?? root?.querySelector<HTMLElement>('[data-create]');
    target?.focus();
  }

  /** The confirmed revoke; a new modal for the next one, since a finished one keeps its buttons off. */
  async function revoke(): Promise<HubResult<unknown>> {
    if (revoking === null) return { ok: true, value: undefined };
    const { id } = revoking;
    const result = await api.revoke(id);
    if (result.ok) {
      const index = keys?.findIndex((key) => key.id === id) ?? -1;
      const rest = keys?.filter((key) => key.id !== id) ?? [];
      keys = rest;
      revoking = null;
      // After the dialog has closed and given focus back to the button that left with its row.
      await tick();
      focusRow(focusAfterRevoke(rest, index));
    }
    return result;
  }
</script>

<div class="keys" bind:this={root}>
  <SettingRow label="New key">
    {#snippet hint()}
      <p class="hint">Anima shows a new key once, when it is made.</p>
    {/snippet}
    <FormButton tabindex={0} data-create onclick={() => { creating = true; }}>Create API key</FormButton>
  </SettingRow>
  <!--
    A hidden row sets the space for the result. Extra keys scroll within it so the VPN section below stays in place (MOT-007).
  -->
  <div class="stack" aria-busy={loading}>
    <ul class="sizer" aria-hidden="true" inert>
      <KeyRow key={PLACEHOLDER_KEY} expiredAt={now()} onRevoke={() => undefined} placeholder />
    </ul>
    {#if loadFailed}
      <div class="failed">
        <span class="mark"><Icon name="carbon:warning" size="sm" /></span>
        <p role="alert">Anima could not load the API keys.</p>
        <FormButton tabindex={0} aria-disabled={retrying} aria-busy={retrying} onclick={retry}>Try again</FormButton>
      </div>
    {:else if keys !== null && keys.length === 0}
      <p class="quiet">No API keys. Programs that call the hub's GraphQL API need one.</p>
    {:else if keys !== null}
      <ul class="list">
        {#each keys as key (key.id)}
          <KeyRow {key} expiredAt={now()} onRevoke={() => { revoking = key; }} />
        {/each}
      </ul>
    {/if}
    {#if skeleton.visible}
      <ul class="skeleton" aria-hidden="true" inert transition:crossFade>
        <KeyRow key={PLACEHOLDER_KEY} expiredAt={now()} onRevoke={() => undefined} placeholder />
      </ul>
    {/if}
  </div>
</div>

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
  .keys {
    display: flex;
    flex-direction: column;
  }

  .stack {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-items: start;
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-strong);
  }

  .stack > * {
    grid-area: 1 / 1;
  }

  .sizer {
    visibility: hidden;
  }

  .list {
    align-self: stretch;
    contain: size;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  /* The skeleton covers what lies under it with the card's own colour. */
  .skeleton {
    background: var(--surface-overlay);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .quiet {
    color: var(--text-secondary);
  }

  .failed {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .mark {
    display: inline-flex;
    color: var(--status-down);
  }

  /* The notice text stays --text (MAT-005); the icon beside it marks the error, never colour alone (UI-003). */
  .failed p {
    flex: 1 1 0;
    min-inline-size: 0;
  }

  .failed :global(button) {
    white-space: nowrap;
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
