<script lang="ts">
  import { tick } from 'svelte';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import type { SaveMark as Mark } from '../draft.svelte';

  /** What was typed: a password, or a private key with its passphrase. */
  export interface TypedSecret {
    readonly password: string;
    readonly privateKey: string;
    readonly passphrase: string;
  }

  interface Props {
    /** "Password" or "Private key". */
    label: string;
    /** The field's id, such as `device-rdp-password`. */
    id: string;
    kind: 'password' | 'key';
    /** K1 `credentialStored`: the client never receives the secret itself (D-4, DEV-011). */
    stored: boolean;
    /** The sentence shown in place of the buttons while the hub cannot store one, such as with the protocol off. */
    unavailable: string | null;
    mark: Mark | undefined;
    /** Stores the typed secret; answers null, or why it was not stored. */
    onSave: (secret: TypedSecret) => Promise<string | null>;
    /** Without it there is no Remove (D-4). */
    onRemove?: (() => Promise<HubResult<unknown>>) | undefined;
  }

  const { label, id, kind, stored, unavailable, mark, onSave, onRemove }: Props = $props();
  let editing = $state(false);
  let busy = $state(false);
  let problem = $state<string | null>(null);
  let typed = $state<TypedSecret>({ password: '', privateKey: '', passphrase: '' });
  const noun = $derived(kind === 'key' ? 'key' : 'password');

  function forget(): void {
    typed = { password: '', privateKey: '', passphrase: '' };
  }

  async function edit(): Promise<void> {
    problem = null;
    editing = true;
    await tick();
    document.getElementById(id)?.focus();
  }

  function cancel(): void {
    forget();
    problem = null;
    editing = false;
  }

  /** D17: the one setting saved on purpose. A refused secret stays typed, with the hub's reason (never the secret). */
  async function save(): Promise<void> {
    if (busy) return;
    busy = true;
    problem = await onSave({ ...typed });
    busy = false;
    if (problem !== null) return;
    forget();
    editing = false;
  }

  async function remove(): Promise<void> {
    if (busy || onRemove === undefined) return;
    busy = true;
    const result = await onRemove();
    busy = false;
    problem = result.ok ? null : result.error.message;
  }
</script>

<div class="secret">
  <span class="label"><label for={editing ? id : undefined}>{label}</label><SaveMark {mark} /></span>
  {#if editing}
    <div class="edit">
      {#if kind === 'key'}
        <textarea {id} rows="5" spellcheck="false" autocomplete="new-password" placeholder="-----BEGIN OPENSSH PRIVATE KEY-----" bind:value={typed.privateKey}></textarea>
        <input id="{id}-passphrase" type="password" autocomplete="new-password" placeholder="Key passphrase, if it has one" aria-label="Key passphrase" bind:value={typed.passphrase} />
      {:else}
        <input
          {id}
          type="password"
          autocomplete="new-password"
          placeholder="New password"
          bind:value={typed.password}
          onkeydown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              void save();
            }
          }}
        />
      {/if}
      <span class="buttons">
        <FormButton size="sm" tone="primary" busy={busy} onclick={() => void save()}>Save {noun}</FormButton>
        <FormButton size="sm" tone="ghost" onclick={cancel}>Cancel</FormButton>
      </span>
    </div>
  {:else}
    <div class="stored">
      {#if stored}
        <span class="state"><Icon name="carbon:locked" size="sm" />Stored</span>
        <span class="quiet">in the hub, encrypted</span>
      {:else}
        <span class="quiet">Not stored</span>
      {/if}
      <span class="grow"></span>
      {#if unavailable !== null}
        <span class="quiet">{unavailable}</span>
      {:else}
        <FormButton size="sm" onclick={() => void edit()}>{stored ? 'Replace' : `Add ${noun}`}</FormButton>
        {#if stored && onRemove !== undefined}
          <FormButton size="sm" tone="ghost" busy={busy} onclick={() => void remove()}>Remove</FormButton>
        {/if}
      {/if}
    </div>
  {/if}
  <p class="problem" role="alert">{problem ?? ''}</p>
</div>

<style>
  .secret {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    grid-column: 1 / -1;
    min-inline-size: 0;
  }

  .label {
    display: flex;
    align-items: baseline;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .stored {
    box-sizing: border-box;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    min-block-size: var(--control-md);
    padding: var(--border-focus) var(--space-1) var(--border-focus) var(--space-3);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--status-up-text);
  }

  .quiet {
    color: var(--text-tertiary);
  }

  .grow {
    flex: 1;
  }

  .edit {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .edit input,
  .edit textarea {
    box-sizing: border-box;
    flex: 1 1 calc(var(--space-16) * 3);
    min-inline-size: 0;
    padding: var(--space-1) var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
    color: var(--text);
    font: inherit;
  }

  .edit input {
    block-size: var(--control-md);
  }

  .edit textarea {
    flex-basis: 100%;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .edit :is(input, textarea):focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .buttons {
    display: inline-flex;
    gap: var(--space-2);
  }

  /* --status-down text is 3.05:1, so the words stay --text and a bar marks the problem (MAT-005). */
  .problem {
    margin: 0;
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid transparent;
    font-size: var(--text-sm);
  }

  .problem:not(:empty) {
    border-inline-start-color: var(--status-down);
  }

  .problem:empty {
    display: none;
  }
</style>
