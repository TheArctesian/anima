<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import type { ThisComputerShell } from '$lib/desktop/agent/shell';
  import Field from '$lib/devices/form/form/Field.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import { flip, measure, type Places } from '$lib/motion/roles/flip';
  import { fade } from '$lib/motion/roles/transitions';
  import { installCommand, refusalMessage } from './command';
  import { clockText } from './countdown';
  import { AgentInstall, type AgentCalls, type ClipboardSink } from './install.svelte';
  import { AGENT_TARGETS, BUILT_RELEASE, targetOfPlatform, type AgentRelease, type AgentTarget } from './release';
  import { ThisComputerInstall } from './this-computer.svelte';
  import ThisComputerButton from './ThisComputerButton.svelte';

  interface Props {
    deviceId: string;
    deviceName: string;
    /** The device's host: when it is this computer's VPN address, the desktop app can install here (R-060). */
    host: string;
    /** The device's saved source: AGENT shows the installed state. */
    source: MetricsSource;
    calls: AgentCalls;
    /** The hub's origin, as the agent must reach it. */
    origin: string;
    now: () => Date;
    release?: AgentRelease | null;
    clipboard?: ClipboardSink;
    /** Runs `run` after `milliseconds`; returns the function that cancels it. */
    later?: (run: () => void, milliseconds: number) => () => void;
    /** True when the page was opened at `#agent`: the panel's heading takes focus. */
    reveal?: boolean;
    /** The desktop shell's calls; `null` in a browser, where only the install command is shown. */
    shell?: ThisComputerShell | null;
  }

  const {
    deviceId,
    deviceName,
    host,
    source,
    calls,
    origin,
    now,
    release = BUILT_RELEASE,
    clipboard = navigator.clipboard,
    later = (run, milliseconds) => {
      const timer = setTimeout(run, milliseconds);
      return () => { clearTimeout(timer); };
    },
    reveal = false,
    shell = null,
  }: Props = $props();

  // The device, hub and clock are fixed for the life of the panel.
  // svelte-ignore state_referenced_locally
  const install = new AgentInstall(calls, deviceId, { now, clipboard, later });
  // svelte-ignore state_referenced_locally
  const here = new ThisComputerInstall(calls, shell, { id: deviceId, host }, origin);
  // The release is fixed for the life of the panel.
  // svelte-ignore state_referenced_locally
  let target = $state<AgentTarget | null>(release === null ? null : targetOfPlatform(navigator.platform, navigator.userAgent));
  let confirming = $state(false);
  /** Each opening of the revoke dialog is a new dialog, so one that finished does not stay finished. */
  let opening = $state(0);
  let section = $state<HTMLElement>();
  let heading = $state<HTMLElement>();
  let codeLabel = $state<HTMLElement>();
  const result = $derived(install.code === null ? null : installCommand({ release, target, code: install.code, origin }));
  const commandText = $derived(result?.ok === true ? result.command.text : null);
  // A read after Close or expiry can show that the code was used: the device is then an AGENT device.
  const isAgent = $derived(source === 'AGENT' || install.status?.metricsSource === 'AGENT');
  const view = $derived(install.code !== null ? 'code' : isAgent ? 'agent' : 'idle');
  const copyShown = $derived(install.copied !== null && install.copied.text === commandText ? install.copied : null);
  /** Everything that changes the panel's height; each change moves what lies below by FLIP (MOT-002, 5.7.3 Disclosure, MOT-006). */
  const layout = $derived(
    [view, copyShown?.count ?? 'no-copy', install.error?.message ?? '', install.expired, install.revoked, install.status?.metricsStale ?? 'unread', install.statusError !== null, result?.ok ?? 'no-command', target ?? 'no-target', here.offered, here.installed, here.error ?? ''].join('|'),
  );

  // What lies below the panel moves by FLIP when the panel changes its height.
  let below: Places | null = null;
  $effect.pre(() => {
    void layout;
    below = measure([...followers()]);
  });
  let previous = 'idle';
  $effect(() => {
    void layout;
    const shown = view;
    if (below !== null) flip(below, { spring: 'default' });
    below = null;
    void tick().then(() => { moveFocus(previous, shown); previous = shown; });
  });

  function* followers(): Generator<HTMLElement> {
    for (let next = section?.nextElementSibling; next; next = next.nextElementSibling) {
      if (next instanceof HTMLElement) yield next;
    }
  }

  /** Focus follows the view: to the code when it opens, back to the button that opened it when it goes (WCAG 2.4.3). */
  function moveFocus(from: string, to: string): void {
    const lost = document.activeElement === document.body || document.activeElement === null || section?.contains(document.activeElement) === true;
    if (!lost || from === to) return;
    if (to === 'code') codeLabel?.focus();
    else if (from === 'code') section?.querySelector<HTMLElement>('[data-agent-start]')?.focus();
  }

  onMount(() => {
    if (source === 'AGENT') void install.loadStatus();
    void here.check();
    if (reveal) heading?.focus();
  });
  onDestroy(() => { here.dispose(); });
  $effect(() => {
    if (install.code === null) return;
    const timer = setInterval(() => { install.tick(); }, 1000);
    return () => { clearInterval(timer); };
  });
</script>

{#snippet starter(label: string)}
  <p>{isAgent ? `Install the agent again on ${deviceName}, for example after a revoke.` : `Install the Anima agent on ${deviceName} to send its metrics.`}</p>
  {#if install.expired}
    <p role="status">The code expired. Make a new one.</p>
  {/if}
  {#if install.error !== null}
    <p role="alert">{install.error.message}</p>
  {/if}
  {#if here.offered}
    <ThisComputerButton install={here} onInstalled={() => void install.loadStatus()} />
  {/if}
  <!-- On this computer the command stays as the way round a failed install, and for an install by hand. -->
  <FormButton data-agent-start onclick={() => void install.makeCode()} aria-disabled={install.busy} aria-busy={install.busy}>
    {install.busy ? 'Making a code…' : here.offered ? 'Show install command' : label}
  </FormButton>
{/snippet}

{#snippet agentState()}
  <p>Metrics come from the agent on {deviceName}. The source is set by the agent.</p>
  <p class="state" role={install.status === null ? undefined : 'status'}>
    {#if install.status === null}
      {install.statusError === null ? 'Reading the agent state.' : 'The agent state could not be read.'}
    {:else if install.revoked}
      Agent revoked. Metrics stay stale until it enrols again or you change the source.
    {:else if install.status.metricsStale}
      Metrics are stale: no readings arrived in the last 30 seconds.
    {:else}
      Metrics are current.
    {/if}
  </p>
  <FormButton tone="danger" onclick={() => { opening += 1; confirming = true; }}>Revoke agent</FormButton>
  {@render starter('Install agent again')}
{/snippet}

{#snippet code(shown: string)}
  <p>
    <span tabindex="-1" bind:this={codeLabel}>Enrolment code</span>
    <code class="code">{shown}</code>
  </p>
  <p class="state timer" role="timer" aria-live="off">Works once and expires in {clockText(install.left)}.</p>
  {#if release !== null}
    <Field label="Computer" field="agent-target" errors={[]}>
      {#snippet control(attributes)}
        <select {...attributes} bind:value={target}>
          {#if target === null}<option value={null} disabled>Choose</option>{/if}
          {#each AGENT_TARGETS as choice (choice.target)}
            <option value={choice.target}>{choice.label}</option>
          {/each}
        </select>
      {/snippet}
    </Field>
  {/if}
  {#if result?.ok === true}
    <pre class="command" aria-label="Install command">{result.command.text}</pre>
    {#if result.command.kind === 'enrol-only'}<p>{result.command.note}</p>{/if}
    <FormButton onclick={() => void install.copy(result.command.text)}>Copy command</FormButton>
    {#if copyShown !== null}
      {#key copyShown.count}
        <p role="status">{copyShown.ok ? 'Copied.' : 'The command could not be copied. Select it and copy it.'}</p>
      {/key}
    {/if}
  {:else if result?.ok === false}
    <p role="alert">{refusalMessage(result.refusal)}</p>
  {/if}
  <FormButton onclick={() => { install.close(); }}>Close</FormButton>
{/snippet}

<section class="agent" id="agent" aria-labelledby="agent-heading" bind:this={section}>
  <h3 id="agent-heading" tabindex="-1" bind:this={heading}>Agent</h3>
  <!-- One-sided on purpose: the outgoing view holds the code and the focused button, so it leaves the document at once (LOG-007, WCAG 2.4.3); the incoming one fades in. -->
  {#key view}
    <div class="body" in:fade>
      {#if view === 'code' && install.code !== null}
        {@render code(install.code)}
      {:else if view === 'agent'}
        {@render agentState()}
      {:else}
        {@render starter('Install agent')}
      {/if}
    </div>
  {/key}
</section>

{#key opening}
  <ConfirmModal
    open={confirming}
    title="Revoke the agent on {deviceName}?"
    body="The agent stops sending metrics on its next push. The device keeps the Agent source, shown as stale, until you change it."
    confirmLabel="Revoke agent"
    onConfirm={async () => {
      const done = await install.revoke();
      if (done.ok) confirming = false;
      return done;
    }}
    onCancel={() => { confirming = false; }}
  />
{/key}

<style>
  .agent {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    padding-block-start: var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  h3 {
    margin: 0;
    font-size: var(--text-md);
  }

  p {
    margin: 0;
    color: var(--text-muted);
  }

  .state {
    font-size: var(--text-sm);
  }

  .code,
  .command {
    font-family: var(--font-mono);
    user-select: all;
  }

  .code {
    overflow-wrap: anywhere;
  }

  .timer {
    font-variant-numeric: tabular-nums;
  }

  h3:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .command {
    box-sizing: border-box;
    inline-size: 100%;
    margin: 0;
    padding: var(--space-3);
    overflow-x: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    background: var(--surface-raised);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: var(--text-sm);
  }

  .body {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    inline-size: 100%;
  }
</style>
