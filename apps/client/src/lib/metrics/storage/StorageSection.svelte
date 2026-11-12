<script lang="ts">
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import type { HubError } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import { hasMetrics } from '$lib/metrics/source/has-metrics';
  import type { StorageDrive } from '../api';
  import { storageRows, summaryText } from './storage';

  interface Props {
    source: MetricsSource;
    /** `null` while the metrics query is on its way or failed; the Metrics section above shows the error. */
    drives: readonly StorageDrive[] | null;
    error: HubError | null;
  }

  const { source, drives, error }: Props = $props();
  const rows = $derived(drives === null ? [] : storageRows(drives));
  const summary = $derived(drives === null || drives.length === 0 ? null : summaryText(drives));
</script>

<!-- After Metrics, a device's drives one row each (MET-029); hidden when metrics are off, as Metrics says why. -->
{#if hasMetrics(source)}
  <section class="overview-card storage" aria-labelledby="overview-storage">
    <h2 id="overview-storage" class="overview-card-title"><Icon name="nf-md-harddisk" />Storage</h2>
    {#if error !== null}
      <p class="quiet">Storage is not available while metrics cannot load.</p>
    {:else if drives === null}
      <p class="quiet">Loading storage</p>
    {:else if summary === null}
      <p class="quiet">No drives reported yet</p>
    {:else}
      <p class="summary">{summary}</p>
      <ul class="rows">
        {#each rows as row (row.key)}
          <li class="drive" class:unmounted={!row.mounted}>
            <div class="title">
              <span class="name">{row.title}</span>
              {#if row.kind !== null}<span class="kind">{row.kind}</span>{/if}
              {#if row.size !== null}<span class="size">{row.size}</span>{/if}
            </div>
            {#if !row.mounted}
              <p class="quiet">Not mounted</p>
            {:else}
              {#each row.mounts as mount (mount.path)}
                <div
                  class="mount"
                  class:degraded={mount.degraded}
                  role={mount.percent === null ? undefined : 'meter'}
                  aria-label={mount.percent === null ? undefined : mount.path}
                  aria-valuemin={mount.percent === null ? undefined : 0}
                  aria-valuemax={mount.percent === null ? undefined : 100}
                  aria-valuenow={mount.percent}
                  aria-valuetext={mount.percent === null ? undefined : mount.degraded ? `${mount.spoken}, degraded` : mount.spoken}
                >
                  <span class="path">{#if row.showPaths}{mount.path}{/if}</span>
                  <span class="value"
                    >{mount.text}{#if mount.degraded}<span class="warn" aria-hidden="true"> ▲</span>{/if}</span
                  >
                  {#if mount.percent !== null}
                    <span class="track" aria-hidden="true"><span class="fill" style:--fill={mount.percent / 100}></span></span>
                  {/if}
                </div>
              {/each}
            {/if}
            {#if row.activity !== null}<p class="activity">{row.activity}</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .storage {
    margin-block-start: var(--space-4);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .summary {
    color: var(--text);
  }

  .quiet,
  .activity {
    color: var(--text-secondary);
  }

  .activity {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .rows {
    display: grid;
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .drive {
    display: grid;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  /* Not mounted: greyed and said in words, never colour alone (MET-029). */
  .unmounted .name,
  .unmounted .kind,
  .unmounted .size {
    color: var(--text-tertiary);
  }

  .title {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-2);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .name,
  .path {
    overflow-wrap: anywhere;
    font-family: var(--font-mono);
  }

  .name {
    color: var(--text);
    font-weight: var(--weight-bold);
  }

  .kind,
  .size {
    color: var(--text-secondary);
  }

  .mount {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: var(--space-2);
    row-gap: var(--space-1);
    min-inline-size: 0;
    font-size: var(--text-xs);
    line-height: var(--leading-normal);
  }

  .path {
    color: var(--text-secondary);
  }

  .value {
    color: var(--text);
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .track {
    grid-column: 1 / -1;
    block-size: var(--space-1);
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
    overflow: hidden;
  }

  .fill {
    display: block;
    block-size: 100%;
    background: var(--accent);
    transform: scaleX(var(--fill));
    transform-origin: left center;
  }

  /* Degraded is also a marker and a bold value, not colour alone (MET-004). */
  .degraded .value {
    font-weight: var(--weight-bold);
  }

  .warn {
    color: var(--status-degraded);
  }

  .degraded .fill {
    background: var(--status-degraded);
  }
</style>
