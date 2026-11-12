<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import type { Delivery, NotificationChannel } from '../api';
  import { failureText } from '../failure';
  import { NEVER_SENT, notificationLabel, outcomeWord, reasonOf, SENDING, shownTime, testOutcome } from './delivery';
  import { kindLabel, targetOf } from './target';

  interface Props {
    channel: NotificationChannel;
    /** Sends a test through the channel (NOT-004); the list records the delivery. */
    onSendTest: () => Promise<HubResult<Delivery>>;
    onEdit: () => void;
    onDelete: () => void;
  }

  const { channel, onSendTest, onEdit, onDelete }: Props = $props();
  const last = $derived(channel.lastDelivery);
  let testing = $state(false);
  /** What the last Send test came to: the delivery's fixed sentence, or a fixed sentence for a refused call. */
  let outcome = $state('');

  async function sendTest(): Promise<void> {
    if (testing) return;
    testing = true;
    outcome = SENDING;
    const result = await onSendTest();
    testing = false;
    outcome = result.ok ? testOutcome(result.value) : failureText(result.error, 'test');
  }
</script>

<li aria-label={channel.name}>
  <div class="head">
    <strong>{channel.name}</strong>
    <span class="kind">{kindLabel(channel.kind)}</span>
  </div>
  <dl>
    <div><dt>Target</dt><dd class="mono">{targetOf(channel.settings)}</dd></div>
    <div><dt>On by default</dt><dd>{channel.onByDefault ? 'Yes' : 'No'}</dd></div>
    <div class="last">
      <dt>Last delivery</dt>
      {#if last === null}
        <dd class="when">{NEVER_SENT}</dd>
        <dd class="reason"></dd>
      {:else}
        <!-- The time and the rest are two lines in a narrow row, so the longest kind and word never wrap a third (MOT-007). -->
        <dd class="when">
          <time datetime={last.at}>{shownTime(last.at)}</time>
          <span class="joint">·</span>
          {notificationLabel(last.kind)} ·
          <span class="state" class:failed={!last.ok}>{outcomeWord(last)}</span>
        </dd>
        <dd class="reason">{reasonOf(last)}</dd>
      {/if}
    </div>
  </dl>
  <p class="outcome" role="status">{outcome}</p>
  <div class="actions">
    <!-- The label stays while the hub works, so the buttons keep their width and focus (MOT-007); aria-busy says it is busy. -->
    <FormButton tabindex={0} aria-label="Send test through {channel.name}" aria-disabled={testing} aria-busy={testing} onclick={sendTest}>Send test</FormButton>
    <FormButton tabindex={0} aria-label="Edit {channel.name}" data-edit={channel.id} onclick={onEdit}>Edit</FormButton>
    <FormButton tabindex={0} tone="danger" aria-label="Delete {channel.name}" data-delete={channel.id} onclick={onDelete}>Delete</FormButton>
  </div>
</li>

<style>
  li {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-strong);
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
  }

  strong {
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .kind {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  dl {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-6);
    margin: 0;
  }

  .last {
    flex: 1 1 100%;
  }

  dt {
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    overflow-wrap: anywhere;
  }

  .mono {
    font-family: var(--font-mono);
  }

  /* The reason's line is kept when a delivery has none, so a test that fails moves nothing (MOT-007). */
  .reason {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    color: var(--text-secondary);
  }

  /* A failure is the word and a bar beside it, never colour alone (UI-003). */
  .failed {
    padding-inline-start: var(--space-1);
    border-inline-start: var(--border-focus) solid var(--status-down);
    font-weight: var(--weight-bold);
  }

  /* What Send test came to keeps one line in a wide row and two in a narrow one, where the longest sentence wraps (MOT-007). */
  .outcome {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  /* Narrower than the longest "time · kind · word" line: the time takes a line of its own, and two are kept. */
  @container (inline-size < 32rem) {
    .when {
      min-block-size: calc(var(--text-sm) * var(--leading-normal) * 2);
    }

    .when time {
      display: block;
    }

    .joint {
      display: none;
    }

    .outcome {
      min-block-size: calc(var(--text-sm) * var(--leading-normal) * 2);
    }
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: end;
    gap: var(--space-2);
  }
</style>
