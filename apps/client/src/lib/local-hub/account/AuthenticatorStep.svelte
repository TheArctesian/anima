<script lang="ts">
  import { onMount } from 'svelte';
  import CodeBoxes from '$lib/access/code/CodeBoxes.svelte';
  import { createLocalAccount, newTotp } from '$lib/desktop/local-hub/shell';
  import CopyText from '$lib/forms/copy/CopyText.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import QrCode from '$lib/qr/QrCode.svelte';
  import { accountFailure, groupedSecret } from './account';

  interface Props {
    heading: string;
    /** The account's name and password from the account step. */
    name: string;
    password: string;
    /** The hub saved the account with this secret; the code it took cannot sign in (FRN-004). */
    onmade: () => void;
    onback: () => void;
  }

  const { heading, name, password, onmade, onback }: Props = $props();
  /** The secret and its URI: on this screen only, never logged or announced, dropped when the account is made (LOG-007). */
  let secret = $state('');
  let uri = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  /** Remounts the boxes after a refusal, so the next code starts in the first box. */
  let attempt = $state(0);
  let title: HTMLElement | undefined = $state();

  onMount(() => {
    title?.focus();
    void makeSecret();
  });

  async function makeSecret(): Promise<void> {
    busy = true;
    problem = null;
    const made = await newTotp(name);
    busy = false;
    if (made.ok) ({ secret, uri } = made.value);
    else problem = made.message;
  }

  async function create(code: string): Promise<void> {
    if (busy || secret === '') return;
    busy = true;
    problem = null;
    const made = await createLocalAccount({ name, password, secret, code });
    busy = false;
    if (made.ok) {
      secret = '';
      uri = '';
      onmade();
      return;
    }
    problem = accountFailure(made.problem);
    attempt += 1;
  }
</script>

<div class="step">
  <h2 tabindex="-1" bind:this={title}>{heading}</h2>
  <p>Scan this with your authenticator app, or type the key. Then enter its code.</p>
  <!-- The code's area keeps its height while the key is made, so nothing below moves when it shows (MOT-007). -->
  <div class="code">
    {#if uri !== ''}
      <div class="pair">
        <!-- The label never holds the secret; the URI is only drawn. -->
        <QrCode text={uri} label="Authenticator code for Anima" />
        <CopyText label="Authenticator key" text={secret} shown={groupedSecret(secret)} />
      </div>
      <!-- The heading has focus when the step shows; the boxes take it only after a refused code. -->
      {#key attempt}
        <CodeBoxes disabled={busy} autofocus={attempt > 0} oncomplete={(code) => void create(code)} />
      {/key}
    {/if}
  </div>
  <p class="status" role="status">{busy ? (uri === '' ? 'Making a key' : 'Saving the account') : ''}</p>
  <p class="problem" role="alert">{problem ?? ''}</p>
  <div class="links">
    <FormButton tone="ghost" size="sm" icon="carbon:arrow-left" aria-disabled={busy} onclick={onback}>Back</FormButton>
    {#if uri === '' && problem !== null}<FormButton size="sm" onclick={() => void makeSecret()}>Retry</FormButton>{/if}
  </div>
</div>

<style>
  .step {
    display: grid;
    gap: var(--space-4);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  h2:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  /* The QR code (three of space-16), the gap and the code boxes. */
  .code {
    display: grid;
    gap: var(--space-4);
    align-content: start;
    min-block-size: calc(var(--space-16) * 3 + var(--space-4) + var(--control-lg));
  }

  .pair {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--space-4);
    align-items: center;
  }

  .status,
  .problem {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  /* The text stays --text; a bar marks the problem (MAT-005). */
  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
    color: var(--text);
  }

  .links {
    display: flex;
    justify-content: space-between;
    gap: var(--space-4);
  }
</style>
