<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Disclosure from '$lib/forms/disclosure/Disclosure.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import type { AccountRow } from '../store.svelte';
  import { healthWords, lastSyncText } from '../health';
  import RunList from '../runs/RunList.svelte';

  interface Props {
    account: AccountRow;
    now: Date;
    /** The hub's words for the action it refused on this account; empty when none. */
    refusal: string;
    working: boolean;
    onSync: () => void;
    onPause: (paused: boolean) => void;
    onDisconnect: () => void;
  }

  const { account, now, refusal, working, onSync, onPause, onDisconnect }: Props = $props();
  const words = $derived(healthWords(account));
  const paused = $derived(account.health === 'PAUSED' && account.pausedBy === 'OPERATOR');
  const pausedByExtension = $derived(account.health === 'PAUSED' && account.pausedBy === 'EXTENSION_OFF');
  let runsOpen = $state(false);
</script>

<SettingRow label={account.label}>
  {#snippet hint()}
    <p class="hint">{account.sourceName}. <span class="health">{words.name}</span>. {words.meaning}</p>
    <p class="hint">{lastSyncText(account.lastSyncAt, now)}{account.syncing ? '. Syncing now' : ''}</p>
    {#if account.lastError !== null}<p class="hint error">Last error: {account.lastError}</p>{/if}
  {/snippet}
  <FormButton size="sm" onclick={onSync} aria-disabled={working || account.syncing} aria-busy={working}>Sync now</FormButton>
  <!-- An account paused because its extension is off is resumed by setting that extension on, not here. -->
  <FormButton
    size="sm"
    onclick={() => {
      onPause(!paused);
    }}
    aria-disabled={working || pausedByExtension}
  >
    {paused ? 'Resume' : 'Pause'}
  </FormButton>
  <FormButton size="sm" tone="danger" onclick={onDisconnect} aria-disabled={working}>Disconnect</FormButton>
  {#snippet message()}
    <p class="refusal" role="alert">{refusal}</p>
  {/snippet}
</SettingRow>
<div class="runs">
  <Disclosure label="Recent syncs" bind:open={runsOpen}>
    <RunList runs={account.runs} {now} />
  </Disclosure>
</div>

<style>
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .health {
    color: var(--text);
  }

  .refusal:not(:empty) {
    margin: 0;
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
    font-size: var(--text-sm);
  }

  .runs {
    padding: 0 var(--space-4) var(--space-3);
  }
</style>
