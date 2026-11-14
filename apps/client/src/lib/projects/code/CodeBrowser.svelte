<script lang="ts">
  import { untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { HubError } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import BranchPick from '../forge/BranchPick.svelte';
  import type { ForgeApi } from '../forge/api';
  import { readWords } from '../forge/list.svelte';
  import type { CodeDirectory, CodeFile } from '../forge/types';
  import { ENTRY_ICONS, crumbs, sizeWords } from './code';
  import FileView from './FileView.svelte';

  interface Props {
    api: ForgeApi;
    repository: string;
    defaultBranch: string;
    now: () => Date;
  }

  const { api, repository, defaultBranch, now }: Props = $props();
  // svelte-ignore state_referenced_locally
  let branch = $state(defaultBranch);
  /** The folder or file open now, from the repository root; empty is the root. */
  let path = $state('');
  let kind = $state<'directory' | 'file'>('directory');
  let directory = $state.raw<CodeDirectory | null>(null);
  let file = $state.raw<CodeFile | null>(null);
  let failure = $state.raw<HubError | null>(null);
  let loading = $state(true);
  let generation = 0;

  async function load(fresh = false): Promise<void> {
    generation += 1;
    const mine = generation;
    loading = true;
    failure = null;
    const result = kind === 'directory' ? await api.directory(repository, branch, path, fresh) : await api.file(repository, branch, path);
    if (mine !== generation) return;
    loading = false;
    if (!result.ok) failure = result.error;
    else if (kind === 'directory') directory = result.value as CodeDirectory;
    else file = result.value as CodeFile;
  }

  $effect(() => {
    void [branch, path, kind];
    untrack(() => void load());
  });

  function go(next: string, nextKind: 'directory' | 'file'): void {
    path = next;
    kind = nextKind;
  }
</script>

<!-- One folder or one file at a time (PRJ-031): the branch, the path back to the root, then the entries or the text. -->
<section class="code" aria-label="Code">
  <div class="bar">
    <BranchPick {api} {repository} value={branch} onChange={(next) => { branch = next; go('', 'directory'); }} />
    <nav class="path" aria-label="Path">
      {#each crumbs(path) as crumb, index (crumb.path)}
        {#if index > 0}<span class="slash" aria-hidden="true">/</span>{/if}
        {#if crumb.path === path}
          <span aria-current="page">{crumb.name}</span>
        {:else}
          <button type="button" onclick={() => go(crumb.path, 'directory')}>{crumb.name}</button>
        {/if}
      {/each}
    </nav>
    {#if kind === 'directory' && directory !== null}
      <span class="read">{readWords(directory.read, now())}</span>
      <FormButton tone="ghost" size="sm" onclick={() => void load(true)}><Icon name="carbon:renew" size="sm" />Refresh</FormButton>
    {/if}
  </div>
  {#if failure !== null}
    <ErrorState message={failure.message} onRetry={() => void load()} />
  {:else if loading}
    <LoadingState label="Loading {kind === 'file' ? 'the file' : 'the folder'}" rows={4} />
  {:else if kind === 'file' && file !== null}
    <FileView {file} />
  {:else if directory !== null}
    <ul class="entries" aria-label="Files">
      {#each directory.entries as entry (entry.path)}
        <li>
          <Icon name={ENTRY_ICONS[entry.kind]} size="sm" />
          {#if entry.kind === 'DIRECTORY' || entry.kind === 'FILE'}
            <button type="button" onclick={() => go(entry.path, entry.kind === 'DIRECTORY' ? 'directory' : 'file')}>{entry.name}</button>
          {:else}
            <span>{entry.name}</span>
          {/if}
          <span class="size">{entry.kind === 'FILE' ? sizeWords(entry.size) : entry.kind === 'SUBMODULE' ? 'Submodule' : entry.kind === 'SYMLINK' ? 'Link' : ''}</span>
        </li>
      {:else}
        <li class="none">This folder is empty.</li>
      {/each}
    </ul>
    {#if directory.partial}<p class="partial">GitHub lists the first 1,000 entries of a folder; this one may have more.</p>{/if}
  {/if}
</section>

<style>
  .code {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
  }

  .path {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    min-inline-size: 0;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .slash,
  .read,
  .size,
  .none,
  .partial {
    color: var(--text-tertiary);
  }

  .read {
    margin-inline-start: auto;
    font-size: var(--text-xs);
  }

  button {
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-secondary);
    font: inherit;
    text-align: start;
  }

  button:hover {
    color: var(--text);
    text-decoration: underline;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .entries {
    margin: 0;
    padding: 0;
    border-block-start: var(--border-thin) solid var(--border-subtle);
    list-style: none;
  }

  .entries li {
    display: grid;
    grid-template-columns: var(--icon-sm) minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-1);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  .entries li button {
    color: var(--text);
  }

  .size {
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  .partial {
    margin: 0;
    font-size: var(--text-sm);
  }
</style>
