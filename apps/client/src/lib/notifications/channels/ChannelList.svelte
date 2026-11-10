<script lang="ts">
  import { tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { crossFade } from '$lib/motion/roles/transitions';
  import type { Delivery, NotificationChannel, NotificationsApi, NotificationTemplateInput } from '../api';
  import ChannelEditor from '../editor/ChannelEditor.svelte';
  import { failureText } from '../failure';
  import ChannelRow from './ChannelRow.svelte';
  import { focusAfterDelete, PLACEHOLDER_CHANNEL, withDelivery, withSaved } from './listing';

  interface Props {
    api: NotificationsApi;
    /** What a channel without its own template sends (NOT-007). */
    defaultTemplate: NotificationTemplateInput;
    /** The hub's placeholders (D-21). */
    placeholders: readonly string[];
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { api, defaultTemplate, placeholders, clock }: Props = $props();
  /** `null` while loading or after a failed load. */
  let channels = $state<readonly NotificationChannel[] | null>(null);
  let loadFailed = $state(false);
  let retrying = $state(false);
  /** The open editor: `channel` is `null` for a new one. */
  let editing = $state<{ readonly channel: NotificationChannel | null } | null>(null);
  let deleting = $state<NotificationChannel | null>(null);
  /** True while the hub deletes: the dialog stays until it answers, so the outcome is never lost unseen. */
  let removing = $state(false);
  let root: HTMLElement | undefined = $state();
  let scroller: HTMLElement | undefined = $state();
  /** The rows whose space the list keeps; more channels scroll inside it. */
  const KEPT_ROWS = ['first', 'second'];
  const loading = $derived(channels === null && !loadFailed);
  // The clock is fixed for the life of the list.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);

  async function load(): Promise<void> {
    const result = await api.channels();
    if (result.ok) {
      channels = result.value;
      loadFailed = false;
    } else loadFailed = true;
  }

  /** Try again: the button stays mounted, and aria-disabled, while the hub answers; on success focus goes to Add channel (NAV-005). */
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
      root?.querySelector<HTMLElement>('[data-add]')?.focus();
    }
  }

  $effect(() => {
    void load();
  });

  async function sendTest(id: string): Promise<HubResult<Delivery>> {
    const result = await api.sendTest(id);
    if (result.ok && channels !== null) channels = withDelivery(channels, id, result.value);
    return result;
  }

  /** A new channel goes at the end of the list, scrolled into view inside it (the scroller is its rows' offset parent). */
  async function saved(channel: NotificationChannel): Promise<void> {
    if (channels === null) return;
    const added = !channels.some((known) => known.id === channel.id);
    channels = withSaved(channels, channel);
    if (!added) return;
    await tick();
    const row = scroller?.querySelector<HTMLElement>(`[data-delete="${CSS.escape(channel.id)}"]`)?.closest('li');
    if (scroller !== undefined && row instanceof HTMLElement) scroller.scrollTop = row.offsetTop + row.offsetHeight - scroller.clientHeight;
  }

  /** Focus for the operator after a delete: the row that took its place, the row before, or Add channel (NAV-005). */
  function focusRow(id: string | null): void {
    const deletes = [...(root?.querySelectorAll<HTMLElement>('.list [data-delete]') ?? [])];
    const target = deletes.find((button) => button.dataset.delete === id) ?? root?.querySelector<HTMLElement>('[data-add]');
    target?.focus();
  }

  /** The confirmed delete; a fixed sentence when the hub refuses it, never the hub's own text. */
  async function remove(): Promise<HubResult<unknown>> {
    if (deleting === null) return { ok: true, value: undefined };
    const { id } = deleting;
    removing = true;
    const result = await api.remove(id);
    removing = false;
    if (!result.ok) return { ok: false, error: { code: 'INTERNAL', message: failureText(result.error, 'delete') } };
    const index = channels?.findIndex((channel) => channel.id === id) ?? -1;
    const rest = channels?.filter((channel) => channel.id !== id) ?? [];
    channels = rest;
    // Only the dialog of this delete closes; one opened for another channel meanwhile stays.
    if (deleting?.id === id) deleting = null;
    // After the dialog has closed and given focus back to the button that left with its row.
    await tick();
    focusRow(focusAfterDelete(rest, index));
    return result;
  }
</script>

<div class="channels" bind:this={root}>
  <div>
    <FormButton tabindex={0} data-add onclick={() => { editing = { channel: null }; }}>Add channel</FormButton>
  </div>
  <!--
    One grid cell of a fixed height: hidden rows keep the space of two channels, the skeleton lies over them, and what
    the hub answers lands in the same place and scrolls inside it. However many channels arrive, are added or are
    deleted, nothing below the list moves (MOT-007).
  -->
  <div class="stack" aria-busy={loading}>
    <ul class="sizer" aria-hidden="true" inert>
      {#each KEPT_ROWS as key (key)}
        <ChannelRow channel={PLACEHOLDER_CHANNEL} onSendTest={() => new Promise(() => undefined)} onEdit={() => undefined} onDelete={() => undefined} placeholder />
      {/each}
    </ul>
    <div class="scroller" bind:this={scroller}>
      {#if loadFailed}
        <div class="failed">
          <p role="alert">Anima could not load the notification channels.</p>
          <div><FormButton tabindex={0} aria-disabled={retrying} aria-busy={retrying} onclick={retry}>Try again</FormButton></div>
        </div>
      {:else if channels !== null && channels.length === 0}
        <p class="quiet">No channels. Add one to hear when a check goes down or comes back up.</p>
      {:else if channels !== null}
        <ul class="list">
          {#each channels as channel (channel.id)}
            <ChannelRow
              {channel}
              onSendTest={() => sendTest(channel.id)}
              onEdit={() => { editing = { channel }; }}
              onDelete={() => { deleting = channel; }}
            />
          {/each}
        </ul>
      {/if}
    </div>
    {#if skeleton.visible}
      <ul class="skeleton" aria-hidden="true" inert transition:crossFade>
        {#each KEPT_ROWS as key (key)}
          <ChannelRow channel={PLACEHOLDER_CHANNEL} onSendTest={() => new Promise(() => undefined)} onEdit={() => undefined} onDelete={() => undefined} placeholder />
        {/each}
      </ul>
    {/if}
  </div>
</div>

{#if editing !== null}
  <ChannelEditor
    {api}
    channel={editing.channel}
    {defaultTemplate}
    {placeholders}
    onSaved={(channel) => {
      void saved(channel);
    }}
    onClose={() => { editing = null; }}
  />
{/if}

{#key deleting?.id}
  <ConfirmModal
    open={deleting !== null}
    title="Delete {deleting?.name ?? ''}?"
    body="Checks that use it stop sending to it. Its stored secrets are deleted with it."
    confirmLabel="Delete {deleting?.name ?? ''}"
    onConfirm={remove}
    onCancel={() => {
      if (!removing) deleting = null;
    }}
  />
{/key}

<style>
  .channels {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .stack {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-items: start;
  }

  .stack > * {
    grid-area: 1 / 1;
  }

  .sizer {
    visibility: hidden;
  }

  /* Adds nothing to the cell's height (the sizer sets it) and fills it, scrolling what does not fit. */
  .scroller {
    position: relative;
    align-self: stretch;
    contain: size;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  .skeleton {
    background: var(--bg);
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
