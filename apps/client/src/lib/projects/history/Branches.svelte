<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import ForgeSection from '../forge/ForgeSection.svelte';
  import type { ForgeApi } from '../forge/api';
  import { ForgeList } from '../forge/list.svelte';
  import type { Branch } from '../forge/types';
  import { shortSha } from '../forge/words';

  interface Props {
    api: ForgeApi;
    repository: string;
    now: () => Date;
  }

  const { api, repository, now }: Props = $props();
  const list = new ForgeList<Branch>((ask) => api.branches(repository, ask), (row) => row.name);

  $effect(() => {
    untrack(() => void list.load());
  });
</script>

<!-- The branches by name, the default one marked, with the commit each points at. -->
<ForgeSection title="Branches" icon="carbon:branch" {list} {now} empty="No branches">
  <ul class="rows" aria-label="Branches">
    {#each list.rows as branch (branch.name)}
      <li>
        <span class="name">{branch.name}</span>
        <span class="marks">
          {#if branch.default}<span class="pill">Default</span>{/if}
          {#if branch.protected}<span class="pill"><Icon name="carbon:locked" size="sm" />Protected</span>{/if}
        </span>
        <span class="sha">{shortSha(branch.sha)}</span>
      </li>
    {/each}
  </ul>
</ForgeSection>

<style>
  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
    font-size: var(--text-sm);
  }

  .name {
    overflow: hidden;
    font-family: var(--font-mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .marks {
    display: flex;
    gap: var(--space-1);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  .sha {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }
</style>
