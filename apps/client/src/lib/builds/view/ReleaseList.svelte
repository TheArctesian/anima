<script lang="ts">
  import { untrack } from 'svelte';
  import type { Builder } from '../api';
  import Disclosure from '$lib/forms/disclosure/Disclosure.svelte';
  import type { JobView, ReleaseView } from './api';
  import ReleaseCard from './card/ReleaseCard.svelte';
  import { attemptsByVersion, isEnded } from './release';

  interface Props {
    /** Already newest first. */
    releases: readonly ReleaseView[];
    builders: readonly Builder[] | null;
    now: Date;
    selectedId: string | null;
    cancelError: { readonly jobId: string; readonly message: string } | null;
    onselect: (job: JobView) => void;
    oncancel: (job: JobView) => void;
  }

  const { releases, ...shared }: Props = $props();
  const groups = $derived(attemptsByVersion(releases));

  function earlierLabel(count: number, version: string): string {
    return count === 1 ? `1 earlier attempt at ${version}` : `${String(count)} earlier attempts at ${version}`;
  }

  /** Earlier attempts start folded away when they have all ended; one still running starts in view. The operator's choice then stays. */
  const settled = (earlier: readonly ReleaseView[]) => earlier.every((release) => release.jobs.every(isEnded));
</script>

<ul class="releases">
  {#each groups as group, index (group.latest.id)}
    <li>
      <ReleaseCard release={group.latest} prominent={index === 0} {...shared} />
      {#if group.earlier.length > 0}
        <div class="earlier">
          <Disclosure label={earlierLabel(group.earlier.length, group.latest.version)} open={untrack(() => !settled(group.earlier))}>
            {#each group.earlier as release, place (release.id)}
              <ReleaseCard {release} attempt={group.earlier.length - place} {...shared} />
            {/each}
          </Disclosure>
        </div>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .releases {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .releases > li {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* Earlier attempts sit under the release that replaced them, quieter than it. */
  .earlier {
    margin-inline-start: var(--space-4);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
