<script lang="ts">
  import { reorderMotion } from '$lib/motion/roles/reorder.svelte';
  import { listRow } from '$lib/motion/roles/transitions';
  import type { SessionRow } from '../api';
  import { tokens, whole } from '../report/units';
  import { costText, sourceName } from '../report/words';
  import { isRecent, lastUseText, repositoryName, spanText } from './sessions';
  import { totalTokens } from './shape';

  interface Props {
    rows: readonly SessionRow[];
    /** How many sessions the hub did not send. */
    leftOut: number;
    /** The host that every repository of the page has, with its `/`; empty for none. */
    host: string;
    /** The time of now, in milliseconds; the page gives a new one each minute, so the words and marks stay true. */
    now: number;
  }

  const { rows, leftOut, host, now }: Props = $props();
  /** A session used again comes to the top: the rows below slide to their places (5.7.3, Reorder). */
  const track = reorderMotion(() => rows.map((row) => row.id).join('\n'));
</script>

<!-- The single sessions, the newest use first (AIS-025). A dot marks a session used in the last 30 minutes; the daemon
     counts every 15 minutes, so the page does not say that a session runs now (AIS-026). -->
<div class="block">
  <div class="over">
    <span class="label">Sessions</span>
    <span class="note">the newest use first; Recent: used in the last 30 minutes</span>
  </div>
  {#if rows.length === 0}
    <p class="none">No session in these days.</p>
  {:else}
    <table>
      <caption class="sr">Sessions, the newest use first</caption>
      <thead>
        <tr>
          <th scope="col">Repository and model</th>
          <th scope="col">Program and device</th>
          <th scope="col">Last use</th>
          <th scope="col" class="n">Prompts</th>
          <th scope="col" class="n">Tokens</th>
          <th scope="col" class="n">Cost</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row, index (row.id)}
          <!-- A new row fades in where it stands; under reduced motion it comes at once (5.7.3, Reorder). -->
          <tr transition:listRow={{ index }} {@attach track(row.id)}>
            <th scope="row">
              <span class="name" title={row.repository ?? undefined}>{repositoryName(row.repository, host)}</span>
              <span class="detail">{row.model ?? 'No model'}</span>
            </th>
            <td>{sourceName(row.source)}<span class="detail">{row.device.name}</span></td>
            <td>
              <span class="when">
                {#if isRecent(row, now)}<span class="recent"><span class="dot" aria-hidden="true"></span>Recent</span>{/if}
                <span>{lastUseText(row.lastUseAt, now)}</span>
              </span>
              <span class="detail">ran {spanText(row.startedAt, row.lastUseAt)}</span>
            </td>
            <td class="n">{whole(row.prompts)}</td>
            <td class="n">{tokens(totalTokens({ ...row, sessions: 1 }))}</td>
            <td class="n">{costText(row)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if leftOut > 0}
      <p class="none">{leftOut} older {leftOut === 1 ? 'session is' : 'sessions are'} not shown. Choose fewer days or a repository.</p>
    {/if}
  {/if}
</div>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: 0;
    /* In a narrow window the table scrolls inside its block, and the page does not. */
    overflow-x: auto;
    container-type: inline-size;
  }

  .over {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-1) var(--space-3);
  }

  .label,
  thead th {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
  }

  .note,
  .none {
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  .none {
    margin: 0;
  }

  table {
    inline-size: 100%;
    border-collapse: collapse;
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    font-size: var(--text-sm);
  }

  th,
  td {
    padding: var(--space-3) var(--space-4);
    text-align: start;
    vertical-align: top;
  }

  tbody tr {
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  tbody th {
    min-inline-size: calc(var(--space-16) * 2.5);
    font-weight: var(--weight-regular);
    white-space: nowrap;
  }

  td {
    white-space: nowrap;
  }

  .name {
    font-weight: var(--weight-medium);
  }

  .detail {
    display: block;
    color: var(--text-secondary);
  }

  .when {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .recent {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--status-up-text);
  }

  .dot {
    display: inline-block;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-up);
  }

  .n {
    inline-size: 1%;
    font-variant-numeric: tabular-nums;
    text-align: end;
    white-space: nowrap;
  }

  td.n {
    font-family: var(--font-mono);
  }

  td.n:not(:last-child) {
    color: var(--text-secondary);
  }

  .sr {
    position: absolute;
    overflow: hidden;
    inline-size: 1px;
    block-size: 1px;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* In a narrow block, Recent stands over the time and the prompts are left out, so the table fits with no scroll; the
     totals above still count the prompts. */
  @container (max-width: 42rem) {
    th,
    td {
      padding-inline: var(--space-2);
    }

    .when {
      flex-direction: column;
      align-items: start;
      gap: 0;
    }

    th:nth-child(4),
    td:nth-child(4) {
      display: none;
    }
  }
</style>
