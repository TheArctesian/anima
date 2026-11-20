<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { loadClientAt } from '$lib/access/navigation';
  import { storeHubAddress } from '$lib/desktop/address';
  import { joinLink, readLink, type Joined } from '$lib/desktop/link/shell';
  import { clearAccessToken } from '$lib/desktop/token';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { markPendingEnrolment } from '$lib/devices/enrollment/pending-enrolment';
  import { enrollmentCode } from '$lib/link/enrollment-code';
  import Icon from '$lib/icons/Icon.svelte';
  import { crossFade } from '$lib/motion/roles/transitions';
  import QrReader from '$lib/qr/QrReader.svelte';
  import { finishJoin, joinHub } from './join';
  import JoinSteps from './JoinSteps.svelte';
  import { waitForHub, type StepId } from './steps';
  import './phone.css';

  /** The invite as pasted: in this field only, never logged; dropped once the hub may have used it (LOG-007, MOB-035). */
  let invite = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  /** The joined device: its key and link port, which are not secrets. */
  let device = $state<Joined | null>(null);
  let step = $state<StepId>('invite');
  let title: HTMLElement | undefined = $state();
  let joining: HTMLElement | undefined = $state();
  let field: HTMLTextAreaElement | undefined = $state();
  let left = false;
  // A shell or browser that cannot read the clipboard shows no Paste button; the field still takes a long-press paste.
  const canPaste = typeof navigator !== 'undefined' && typeof navigator.clipboard?.readText === 'function';

  const after = {
    clearToken: clearAccessToken,
    saveAddress: (origin: string) => {
      storeHubAddress(localStorage, origin);
    },
  };

  onMount(() => {
    title?.focus();
  });

  onDestroy(() => {
    left = true;
  });

  async function pasteInvite(): Promise<void> {
    try {
      invite = (await navigator.clipboard.readText()).trim();
    } catch {
      // The system refused the clipboard: the field takes the paste instead.
    }
    field?.focus();
  }

  function readInvite(text: string): boolean {
    const value = enrollmentCode(text);
    if (value === null) return false;
    invite = value;
    problem = null;
    return true;
  }

  async function join(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    busy = true;
    problem = null;
    const sent = invite;
    invite = '';
    step = 'link';
    const outcome = await joinHub(sent, { join: joinLink, ...after });
    if (!outcome.joined) {
      busy = false;
      problem = outcome.message;
      step = 'invite';
      // A refusal before the hub saw the invite leaves it usable: the field gets it back.
      if (outcome.inviteKept) invite = sent;
      return;
    }
    device = outcome.device;
    await tick();
    joining?.focus();
    await afterJoin(outcome.finished.ok ? null : outcome.finished.message);
  }

  /** The steps after the link started: the address is saved (else `saved` says why), then the hub must answer. */
  async function afterJoin(saved: string | null): Promise<void> {
    busy = true;
    problem = saved;
    if (saved === null) {
      step = 'hub';
      problem = await waitForHub({ read: readLink, pause: (ms) => new Promise((done) => setTimeout(done, ms)), stopped: () => left });
      if (problem === null) step = 'sign-in';
    }
    busy = false;
  }

  /** The invite is spent: Retry repeats only the steps after the join. */
  async function retry(): Promise<void> {
    if (busy || device === null) return;
    const finished = await finishJoin(device, after);
    await afterJoin(finished.ok ? null : finished.message);
  }

  async function signInToHub(): Promise<void> {
    if (busy) return;
    busy = true;
    await markPendingEnrolment(sessionStorage).catch(() => undefined);
    if (!left) loadClientAt('/sign-in');
  }
</script>

<svelte:head><title>{device === null ? 'Join with an invite' : 'Joining the hub'} – Anima.ing</title></svelte:head>

<!-- Frame 2 of docs/mockup/mobile/index.html: "Paste invite", then "Joining" with one mark for each step (MOB-033). -->
<div class="stack" data-join>
{#if device === null}
  <form onsubmit={join} novalidate transition:crossFade>
    <a class="back" href={resolve('/welcome/hub')}><Icon name="carbon:chevron-left" size="lg" />Back</a>
    <h1 tabindex="-1" bind:this={title}>Join with an invite</h1>
    <QrReader onread={readInvite} label="Scan enrollment QR" />
    <div class="invite-label">
      <label for="invite">Invite</label>
      {#if canPaste}<FormButton icon="carbon:paste" onclick={() => void pasteInvite()}>Paste</FormButton>{/if}
    </div>
    <p class="fine" id="invite-help">Scan the enrollment QR or paste the full invite. Then sign in to the hub.</p>
    <textarea
      id="invite"
      rows="4"
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      aria-describedby="invite-help invite-problem"
      aria-invalid={problem === null ? undefined : 'true'}
      bind:value={invite}
      bind:this={field}
    ></textarea>
    <p id="invite-problem" class="problem" role="alert">{problem ?? ''}</p>
    <FormButton type="submit" tone="primary" size="lg" {busy}>Join hub</FormButton>
    <p class="fine">Works once. Not kept.</p>
  </form>
{:else}
  <div class="joining" transition:crossFade>
    <h1 tabindex="-1" bind:this={joining}>Joining the hub</h1>
    <JoinSteps current={step} failed={problem !== null} notes={{ link: `Key ${device.fingerprint}` }} />
    <p class="problem" role="alert">{problem ?? ''}</p>
    <div class="end">
      {#if problem !== null}
        <FormButton tone="primary" size="lg" {busy} onclick={() => void retry()}>Retry</FormButton>
      {:else if step === 'sign-in'}
        <FormButton tone="primary" size="lg" {busy} onclick={() => void signInToHub()}>Sign in</FormButton>
      {:else}
        <FormButton tone="ghost" size="lg" onclick={() => void goto(resolve('/welcome/hub'))}>Cancel</FormButton>
      {/if}
    </div>
  </div>
{/if}
</div>

<style>
  .stack {
    display: grid;
  }

  form,
  .joining {
    grid-area: 1 / 1;
    display: grid;
    gap: calc(var(--space-2) + var(--border-focus));
    align-content: start;
  }

  form h1,
  .joining h1 {
    margin: var(--space-1) 0 var(--space-2);
    font-size: var(--text-2xl);
    font-weight: var(--weight-regular);
  }

  .back {
    display: inline-flex;
    gap: var(--border-focus);
    align-items: center;
    justify-self: start;
    min-block-size: var(--target-min-touch);
    margin-inline-start: calc(var(--space-2) * -1);
    padding-inline: var(--space-1) var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-lg);
    text-decoration: none;
  }

  .invite-label {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .invite-label :global(button) {
    min-block-size: var(--target-min-touch);
  }

  label {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .fine {
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  /* 16 px machine text: iOS does not zoom a field at that size (MOB-012). */
  textarea {
    box-sizing: border-box;
    padding: var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-lg);
    word-break: break-all;
    resize: vertical;
  }

  textarea[aria-invalid='true'] {
    border-color: var(--status-down);
  }

  .problem {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    font-size: var(--text-sm);
  }

  form .problem:empty {
    position: absolute;
  }

  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }

  .end {
    display: grid;
  }

  @media (max-height: 30rem) {
    form {
      gap: var(--space-2);
    }

    form h1 {
      margin-block: 0;
    }

    textarea {
      block-size: clamp(var(--space-12), 17svh, calc(var(--space-16) + var(--space-4)));
      resize: none;
    }
  }

  /* On a phone the button of the joining view sits low, under the thumb (frame 2). */
  @media (max-width: 767.98px) {
    .end {
      position: fixed;
      inset-inline: var(--space-4);
      inset-block-end: calc(var(--safe-block-end) + var(--space-3));
    }
  }

  :is(textarea, a, h1):focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
