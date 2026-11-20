<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { loadClientAt } from '$lib/access/navigation';
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { HostedParts } from '$lib/hosted/choose';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { isMobile } from '$lib/platform/mobile';
  import { capabilities } from '$lib/platform/platform';
  import { FIRST, nextStep, stepText, type HostedEvent, type HostedStep } from './hosted';
  import { runStep } from './run';
  import './phone.css';

  interface Props {
    /** The cloud and the join: the stub, or the stand-in for tests and `VITE_ANIMA_HOSTED=stand-in`. */
    parts: HostedParts;
    /** Opens the device list once the hub is joined; it offers "Add this computer" itself (R-060). */
    ondone?: () => void;
    /** Waits between polls; tests pass their own (CQ-021). */
    wait?: (ms: number) => Promise<void>;
  }

  const { parts, ondone = () => loadClientAt('/devices'), wait = (ms) => new Promise((done) => setTimeout(done, ms)) }: Props = $props();

  let step = $state<HostedStep>(FIRST);
  let gone = false;
  /** Each run of a step has a number; the answer of an older run is dropped. */
  let turn = 0;
  const text = $derived(stepText(step, isMobile(navigator) ? 'this device' : 'this computer', capabilities().accountSession));
  const canLeave = $derived(step.kind === 'start' || step.kind === 'waiting' || step.kind === 'failed');

  onMount(() => {
    void run(step);
    return () => {
      gone = true;
    };
  });

  function go(event: HostedEvent): void {
    step = nextStep(step, event);
    if (step.kind === 'done') ondone();
    else void run(step);
  }

  async function run(current: HostedStep): Promise<void> {
    turn += 1;
    const mine = turn;
    // A page that has gone stops polling: its wait never ends.
    const event = await runStep(current, parts, (ms) => (gone ? new Promise<void>(() => undefined) : wait(ms)));
    if (event !== null && mine === turn && !gone) go(event);
  }

  async function reopen(): Promise<void> {
    if (step.kind !== 'waiting') return;
    const opened = await parts.account.openPage(step.code.page);
    step = nextStep(step, { kind: 'opened', opened: opened.ok });
  }

  function act(): void {
    if (step.kind === 'waiting') void reopen();
    else if (step.kind === 'failed') go({ kind: 'retry' });
  }

  /** Each step's heading takes focus when the step shows (NAV-005). */
  function focused(node: HTMLElement): void {
    node.focus();
  }
</script>

<h1>Sign in</h1>

<!-- One step at a time; the steps cross-fade in one cell (MOT-008). -->
<div class="steps" data-hosted>
  {#key step.kind}
    <div class="step" class:failed={step.kind === 'failed'} transition:crossFade>
      <h2 tabindex="-1" {@attach focused}>{text.title}</h2>
      <p class="line" role={step.kind === 'failed' ? 'alert' : 'status'}>{text.line}</p>
      {#if step.kind === 'waiting'}
        <p class="code" aria-label="Code {step.code.code.split('').join(' ')}">{step.code.code}</p>
      {/if}
      <div class="actions">
        {#if canLeave}<a href={resolve('/welcome')}>Back</a>{/if}
        {#if text.action !== null}
          <FormButton tone={step.kind === 'waiting' ? 'secondary' : 'primary'} onclick={act}>{text.action}</FormButton>
        {/if}
      </div>
    </div>
  {/key}
</div>

<style>
  .steps {
    display: grid;
  }

  .step {
    grid-area: 1 / 1;
    display: grid;
    gap: var(--space-2);
    align-content: start;
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .line {
    color: var(--text-secondary);
  }

  .failed .line {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }

  .code {
    font-family: var(--font-mono);
    font-size: var(--text-2xl);
    letter-spacing: var(--tracking-label);
  }

  /* The row keeps its height while a step works, so nothing moves when its button shows (MOT-007). */
  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-block-size: var(--target-min-desktop);
    margin-block-start: var(--space-2);
  }

  .actions :global(button) {
    margin-inline-start: auto;
  }

  .actions a {
    display: inline-flex;
    align-items: center;
    min-block-size: var(--target-min-desktop);
    min-inline-size: var(--target-min-desktop);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  :is(a, h2):focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
