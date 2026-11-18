<script lang="ts">
  import { untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import { describeAttempt, hubPageAddress, NO_ATTEMPT, RESTARTED, TAB_BLOCKED } from './attempt';
  import { githubConnectApi } from './api';
  import { linkOpening, type LinkOpening } from './browser';
  import { GithubConnectStore } from './github-connect.svelte';

  // The three-press connection (PRJ-039): the operator presses Connect GitHub here, then Create and Install on GitHub.
  interface Props {
    /** Makes the opening for a link. It is called inside the click; tests pass their own. */
    opener?: () => LinkOpening;
    onKeyFile: () => void;
  }

  const { opener = linkOpening, onKeyFile }: Props = $props();
  // The hub is fixed for the life of the row.
  const store = untrack(() => new GithubConnectStore(githubConnectApi(useHub())));
  const view = $derived(store.attempt !== null ? describeAttempt(store.attempt) : store.restarted ? RESTARTED : NO_ATTEMPT);
  const pageUrl = $derived(store.blocked && store.attempt !== null ? hubPageAddress(store.attempt.pageUrl) : null);

  $effect(() => {
    void store.ensureLoaded();
    return () => {
      store.stop();
    };
  });

  function openOnGithub(url: string): void {
    const opening = opener();
    void opening.open(url);
  }
</script>

<div class="connect">
  {#if view.line !== ''}
    <p class="line" role="status">{view.line}</p>
  {/if}
  {#if pageUrl !== null}
    <p class="line">
      {TAB_BLOCKED}
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the hub's own page, not a route of this app -->
      <a href={pageUrl} target="_blank" rel="noopener noreferrer">Open connect page</a>
    </p>
  {/if}
  {#if store.refusal !== ''}
    <p class="line" role="alert">{store.refusal}</p>
  {/if}
  <div class="actions">
    {#if view.start}
      <FormButton busy={store.busy} onclick={() => void store.start(opener())}>Connect GitHub</FormButton>
    {/if}
    {#if view.installUrl !== null}
      {@const url = view.installUrl}
      <FormButton tone="primary" onclick={() => { openOnGithub(url); }}>Install on GitHub</FormButton>
    {/if}
    {#if view.continueInBrowser}
      <FormButton busy={store.busy} onclick={() => void store.continueInBrowser(opener())}>Continue in browser</FormButton>
    {/if}
    {#if view.appSettingsUrl !== null}
      {@const url = view.appSettingsUrl}
      <FormButton onclick={() => { openOnGithub(url); }}>Delete App on GitHub</FormButton>
    {/if}
    {#if view.cancel}
      <FormButton tone="ghost" onclick={() => void store.cancel()}>Cancel</FormButton>
    {/if}
    <FormButton tone="ghost" icon="simple-icons:github" onclick={onKeyFile}>Use key file</FormButton>
  </div>
</div>

<style>
  .connect {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .line {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    overflow-wrap: anywhere;
  }
</style>
