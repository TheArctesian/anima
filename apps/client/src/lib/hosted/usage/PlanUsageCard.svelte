<script lang="ts">
  import { formatBytes } from '$lib/metrics/units';
  import {
    ACCOUNT_PAGE,
    ESTIMATE_NOTE,
    NOT_AVAILABLE,
    NOT_CHARGED,
    OWES,
    SIGNED_OUT,
    amountText,
    figuresTime,
    hoursText,
    planDiskText,
    type UsageAnswer,
  } from './usage';

  interface Props {
    /** Asks the account for its figures (`read.ts`); the hub holds none of them (HMD-073 in form B). */
    read: () => Promise<UsageAnswer>;
    /** The bytes of the hub's own data, which the hub measures (HMD-072); `null` before its daemon sent them. */
    dataBytes: number | null;
    now: () => Date;
  }

  const { read, dataBytes, now }: Props = $props();
  let answer = $state<UsageAnswer | null>(null);

  $effect(() => {
    let current = true;
    void read().then((value) => {
      if (current) answer = value;
    });
    return () => {
      current = false;
    };
  });
</script>

<!-- The use of the plan on the page of the hub (HMD-073, task HM-T38 of hosted-mode.md). "Data of your hub" and
     "Disk used of the plan" are two figures and never one: the first is what the hub measures, the second what the
     quota of the plan counts, the system of the hub included. -->
<section class="overview-card plan-usage" aria-labelledby="overview-plan-usage" aria-busy={answer === null}>
  <h2 id="overview-plan-usage" class="overview-card-title">Plan and usage</h2>
  <dl>
    <div><dt>Data of your hub</dt><dd>{dataBytes === null ? 'Not measured yet' : formatBytes(dataBytes)}</dd></div>
    {#if answer === null}
      <div><dt>Plan</dt><dd class="quiet">Loading…</dd></div>
    {:else if answer.state === 'figures'}
      {@const usage = answer.usage}
      <div><dt>Plan</dt><dd>{usage.plan.name}</dd></div>
      <div><dt>Disk used of the plan</dt><dd>{planDiskText(usage.disk)}</dd></div>
      <div><dt>Awake</dt><dd>{hoursText(usage.awakeSeconds)}</dd></div>
      <div><dt>Asleep</dt><dd>{hoursText(usage.asleepSeconds)}</dd></div>
      <div>
        <dt>This month</dt>
        <dd>{usage.amount === null || !usage.plan.charged ? NOT_CHARGED : amountText(usage.amount)}</dd>
      </div>
    {/if}
  </dl>
  {#if answer?.state === 'figures'}
    {#if answer.usage.amount !== null && answer.usage.plan.charged}<p class="note">{ESTIMATE_NOTE}</p>{/if}
    {#if answer.usage.owes}
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the account page, not a route of this app -->
      <p class="note">{OWES} <a href={ACCOUNT_PAGE} target="_blank" rel="noreferrer">Open your account</a></p>
    {/if}
    <p class="quiet">{figuresTime(answer.usage.at, now())}</p>
  {:else if answer?.state === 'signed-out'}
    <p class="quiet">{SIGNED_OUT}</p>
  {:else if answer?.state === 'unavailable'}
    <p class="quiet">{NOT_AVAILABLE}</p>
  {/if}
</section>

<style>
  dl {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-sm);
  }

  dl > div {
    display: grid;
    grid-template-columns: calc(var(--space-16) + var(--space-8)) minmax(0, 1fr);
    gap: var(--space-3);
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    margin: 0;
    color: var(--text);
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }

  p {
    margin: var(--space-2) 0 0;
    font-size: var(--text-sm);
  }

  .note {
    color: var(--text-secondary);
  }

  .quiet {
    color: var(--text-tertiary);
  }
</style>
