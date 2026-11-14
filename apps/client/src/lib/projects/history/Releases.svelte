<script lang="ts">
  import { untrack } from 'svelte';
  import { dataAge } from '$lib/hub/hub';
  import ForgeSection from '../forge/ForgeSection.svelte';
  import type { ForgeApi } from '../forge/api';
  import { ForgeList } from '../forge/list.svelte';
  import type { RepositoryRelease } from '../forge/types';

  interface Props {
    api: ForgeApi;
    repository: string;
    now: () => Date;
  }

  const { api, repository, now }: Props = $props();
  const list = new ForgeList<RepositoryRelease>((ask) => api.releases(repository, ask), (row) => row.id);

  $effect(() => {
    untrack(() => void list.load());
  });
</script>

<!-- Releases on GitHub, newest first; not releases of Anima. -->
<ForgeSection title="Releases" icon="carbon:tag" {list} {now} empty="No releases">
  <ul class="rows" aria-label="Releases">
    {#each list.rows as release (release.id)}
      <li>
        <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub -->
        <a class="tag" href={release.url} target="_blank" rel="noreferrer">{release.tag}</a>
        <span class="name">{release.name ?? ''}</span>
        <span class="quiet">
          {release.draft ? 'Draft' : release.prerelease ? 'Pre-release' : ''}
          {dataAge(new Date(release.publishedAt ?? release.createdAt), now())}
        </span>
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
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: baseline;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
    font-size: var(--text-sm);
  }

  .tag {
    color: var(--text);
    font-family: var(--font-mono);
    text-decoration: none;
  }

  .tag:hover {
    text-decoration: underline;
  }

  .name {
    overflow: hidden;
    color: var(--text-secondary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quiet {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }
</style>
