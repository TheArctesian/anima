<script lang="ts">
  import type { AgentAccount, DeviceSourceRow } from '../api';
  import type { Count } from '../query';
  import { sourceName } from '../report/words';
  import { deviceShares, limitFill, limitLevel, limitText, windowWords } from './accounts';
  import { lastUseText } from './sessions';

  interface Props {
    accounts: readonly AgentAccount[];
    /** The use of each program on each device in the days of the page. */
    shares: readonly DeviceSourceRow[];
    /** What the share of a device counts. */
    count: Count;
    /** The time of now, in milliseconds; the page gives a new one each minute. */
    now: number;
  }

  const { accounts, shares, count, now }: Props = $props();
</script>

<!-- One card for each account (AIS-022 as R-313 changed it): its limit windows, which its devices share, and the share of
     each device. The hub knows an account by its key only; the name of the account stays on each device. -->
<div class="block">
  <div class="over">
    <span class="label">By account</span>
    <span class="note">one limit for each account, shared by its devices; the name of the account stays on each device</span>
  </div>
  {#if accounts.length === 0}
    <p class="none">No account to show. The devices do not send their accounts yet.</p>
  {:else}
  <ul class="cards">
    {#each accounts as account (account.key)}
      <li class="card" aria-label={`${sourceName(account.source)} account ${account.shortKey}`}>
        <div class="head">
          <span class="title"><b>{sourceName(account.source)}</b>{#if account.plan !== null}<span class="quiet">{account.plan}</span>{/if}</span>
          <span class="small quiet">account <span class="mono">{account.shortKey}</span> · reported {lastUseText(account.reportedAt, now).toLowerCase()}</span>
        </div>
        {#if account.limits.length === 0}
          <p class="quiet">The program reports no limit yet.</p>
        {:else}
          {#each account.limits as limit (limit.window)}
            {@const level = limitLevel(limit)}
            <div class="limit">
              <div class="line"><span>{windowWords(limit.window)}</span><span class="quiet" class:reached={level === 'reached'}>{limitText(limit, now)}</span></div>
              <span class="track" role="meter" aria-label={`${windowWords(limit.window)} used`} aria-valuemin="0" aria-valuemax="100" aria-valuenow={limitFill(limit)}>
                <i class={level} style:inline-size="{limitFill(limit)}%"></i>
              </span>
            </div>
          {/each}
        {/if}
        <div class="devices">
          <span class="small quiet">Each device's share of this program's {count} in these days</span>
          {#each deviceShares(account, shares, count) as device (device.id)}
            <div class="device">
              <span class="name">{device.name}</span>
              {#if device.percent === null}
                <span class="quiet share-none">no use</span>
              {:else}
                <span class="share" aria-hidden="true"><i style:inline-size="{device.percent}%"></i></span>
                <span class="percent">{device.percent} %</span>
              {/if}
            </div>
          {/each}
        </div>
      </li>
    {/each}
  </ul>
  {/if}
</div>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  .over {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-1) var(--space-3);
  }

  .label {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
  }

  .note,
  .small {
    font-size: var(--text-xs);
  }

  .note,
  .quiet {
    color: var(--text-secondary);
  }

  .none {
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(var(--space-16) * 4.5), 1fr));
    gap: var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    font-size: var(--text-sm);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title {
    display: flex;
    gap: var(--space-1);
    align-items: baseline;
  }

  b {
    font-weight: var(--weight-medium);
  }

  p {
    margin: 0;
  }

  .mono,
  .percent {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .limit {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: var(--space-1) var(--space-3);
  }

  .reached {
    color: var(--status-down-text);
  }

  .track,
  .share {
    display: block;
    border-radius: var(--radius-sm);
    background: var(--surface-selected);
  }

  .track {
    block-size: calc(var(--space-1) * 1.5);
  }

  .track i,
  .share i {
    display: block;
    block-size: 100%;
    border-radius: var(--radius-sm);
  }

  .track i.normal {
    background: var(--chart-series-1);
  }

  .track i.high {
    background: var(--status-warning);
  }

  .track i.reached {
    background: var(--status-down);
  }

  .devices {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding-block-start: var(--space-2);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .device {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--space-16) calc(var(--space-16) * 0.75);
    gap: var(--space-2);
    align-items: center;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .share {
    block-size: var(--space-1);
  }

  .share i {
    background: var(--chart-series-2);
  }

  .percent {
    color: var(--text-secondary);
    text-align: end;
  }

  .share-none {
    grid-column: 2 / -1;
    text-align: end;
  }
</style>
