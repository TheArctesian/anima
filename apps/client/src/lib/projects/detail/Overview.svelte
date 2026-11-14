<script lang="ts">
  import KeyValueList from '$lib/page/KeyValueList.svelte';
  import type { Pair } from '$lib/page/pair';
  import Section from '$lib/page/Section.svelte';
  import type { Project } from '../api';
  import RepositoryRow from '../RepositoryRow.svelte';
  import ResourceRow from '../ResourceRow.svelte';
  import { primaryRepository } from '../words';

  interface Props {
    project: Project;
    onLeaveOut: (repositoryId: string) => void;
    onRestore: (repositoryId: string) => void;
    onConfirm: (resourceId: string) => void;
  }

  const { project, onLeaveOut, onRestore, onConfirm }: Props = $props();
  const primary = $derived(primaryRepository(project));
  const about = $derived<readonly Pair[]>(
    primary === undefined
      ? []
      : [
          { label: 'Repository', value: primary.fullName, icon: 'simple-icons:github', mono: true },
          { label: 'Visibility', value: primary.private ? 'Private' : 'Public', icon: 'carbon:locked' },
          ...(primary.fork ? [{ label: 'Fork', value: 'Yes', icon: 'carbon:branch' } as const] : []),
          ...(primary.description == null || primary.description === '' ? [] : [{ label: 'About', value: primary.description, icon: 'carbon:information' } as const]),
        ],
  );
</script>

<!-- The Overview tab (PRJ-030, UR-7): where the project runs and its repositories, with the facts beside them. -->
<div class="overview">
  <div class="main">
    <Section title="Where it runs" icon="carbon:cloud-upload" lede="A sign in the repository proposes a place; it counts once you confirm it.">
      {#if project.resources.length === 0}
        <p class="none">Not running anywhere. Anima looks for a wrangler.toml, vercel.json, Dockerfile, Compose file or tauri.conf.json.</p>
      {:else}
        <ul>
          {#each project.resources as resource (resource.id)}
            <ResourceRow {resource} {onConfirm} />
          {/each}
        </ul>
      {/if}
    </Section>
    <Section title="Repositories" icon="simple-icons:github">
      <ul>
        {#each project.repositories as repository (repository.id)}
          <RepositoryRow {repository} {onLeaveOut} {onRestore} />
        {/each}
      </ul>
    </Section>
  </div>
  {#if about.length > 0}
    <aside aria-label="About this project">
      <Section title="About" icon="carbon:information">
        <KeyValueList pairs={about} />
      </Section>
    </aside>
  {/if}
</div>

<style>
  .overview {
    display: grid;
    grid-template-columns: minmax(0, 1fr) calc(var(--space-16) * 4.5);
    gap: var(--space-8) var(--space-12);
    align-items: start;
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    min-inline-size: 0;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .none {
    margin: 0;
    padding-block: var(--space-3);
    border-block: var(--border-thin) solid var(--border-subtle);
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  @media (max-width: 1100.98px) {
    .overview {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
