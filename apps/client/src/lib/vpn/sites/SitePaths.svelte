<script lang="ts">
  import { resolve } from '$app/paths';
  import { listRow } from '$lib/motion/roles/transitions';
  import { press } from '$lib/motion/roles/press';
  import type { SitePath } from '../api';
  import { pathLabel, stateLabel } from '../picture';

  interface Props {
    sites: readonly SitePath[];
  }

  const { sites }: Props = $props();

  const TONES = { SERVING: 'up', OFFLINE: 'down', NO_ROUTER: 'unknown' } as const;
</script>

<!-- One row for each site: how the hub reaches it, in words beside a dot (VPN-009, UI-003). -->
<ul class="sites" aria-label="Sites">
  {#each sites as entry, index (entry.site.id)}
    <li class="row" transition:listRow={{ index }} aria-label={entry.site.name}>
      <span class="name">{entry.site.name}</span>
      <span class="subnet">{entry.site.subnet}</span>
      <span class="state {TONES[entry.state]}"><span class="dot" aria-hidden="true"></span>{stateLabel(entry.state)}</span>
      <span class="path">{pathLabel(entry.path, entry.relayRegion)}</span>
      {#if entry.state === 'SERVING' && !entry.hasPingCheck}
        {#if entry.router?.device}
          <a class="hint" href="{resolve('/devices')}/{encodeURIComponent(entry.router.device.id)}?tab=checks" tabindex="0" {@attach press()}>Add a ping check</a>
        {:else}
          <span class="hint">The router is not a device in Anima</span>
        {/if}
      {/if}
    </li>
  {/each}
</ul>

<style>
  .sites {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto;
    gap: var(--space-1) var(--space-4);
    align-items: center;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  /* A narrow card keeps the name and the state on the first line; the subnet and the path each take a full line, so a wide state cannot squeeze them. */
  @container (max-inline-size: 28rem) {
    .row {
      grid-template-columns: minmax(0, 1fr) auto;
    }

    .name {
      grid-column: 1;
      grid-row: 1;
    }

    .state {
      grid-column: 2;
      grid-row: 1;
    }

    .subnet {
      grid-column: 1 / -1;
      grid-row: 2;
    }

    .path {
      grid-column: 1 / -1;
      grid-row: 3;
    }
  }

  .name {
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  .subnet,
  .path {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .dot {
    flex: none;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--tone);
  }

  .up {
    --tone: var(--status-up);
  }

  .down {
    --tone: var(--status-down);
  }

  .unknown {
    --tone: var(--status-unknown);
  }

  .hint {
    grid-column: 1 / -1;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  a.hint {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    min-block-size: var(--target-min-desktop);
    color: var(--accent);
  }

  a.hint:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
