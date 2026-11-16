<script lang="ts">
  import { onMount } from 'svelte';
  import { installLocalHub, readLocalAccount, readLocalHub, type LocalHubStatus } from '$lib/desktop/local-hub/shell';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { isMac } from '$lib/platform/mac';
  import { linkText, pollUntilSettled, progressOf, stateText } from '$lib/local-hub/state';

  interface Props {
    /** The hub is ready; `accountExists` when an operator account is already there (FRN-002). */
    onready: (accountExists: boolean) => void;
    /** Waits between reads; tests pass their own (CQ-021). */
    wait?: (ms: number) => Promise<void>;
  }

  const { onready, wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms)) }: Props = $props();
  const mac = isMac(navigator.userAgent);
  let status = $state<LocalHubStatus | null>(null);
  let problem = $state<string | null>(null);
  let working = $state(false);
  let gone = false;

  onMount(() => {
    void install();
    return () => {
      gone = true;
    };
  });

  async function install(): Promise<void> {
    working = true;
    problem = null;
    const before = await readLocalHub();
    const ready = before.ok && before.value.installed && progressOf(before.value) === 'ready';
    if (!ready) {
      const installed = await installLocalHub();
      if (!installed.ok) return stop(installed.message);
    }
    const last = await pollUntilSettled({ read: readLocalHub, wait, seen: (seen) => (status = seen), stopped: () => gone });
    if (last === null) return;
    if (!last.ok) return stop(last.message);
    if (progressOf(last.value) !== 'ready') return stop(null);
    const account = await readLocalAccount();
    if (!account.ok) return stop(account.message);
    onready(account.value !== null);
  }

  function stop(message: string | null): void {
    working = false;
    problem = message;
  }
</script>

<p>Anima.ing installs the hub, its database, its gateway and its link on this computer, then starts them. The first start can take a few minutes.</p>
<div class="state" role="status" aria-live="polite">
  {#if status !== null}
    <p>{stateText(status, mac)}</p>
    {#if linkText(status) !== null}<p class="link">{linkText(status)}</p>{/if}
  {:else if working}
    <p>Installing.</p>
  {/if}
</div>
<p class="problem" role="alert">{problem ?? ''}</p>
{#if !working}
  <FormButton tone="primary" onclick={() => void install()}>Try again</FormButton>
{/if}

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .state {
    display: grid;
    gap: var(--space-1);
    min-block-size: calc(2 * var(--text-md) * var(--leading-normal));
  }

  .link {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
