<script lang="ts">
  import { flushSync, untrack } from 'svelte';
  import KeySecret from '$lib/api-keys/KeySecret.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { clockText } from '$lib/metrics/agent/countdown';
  import { fade } from '$lib/motion/roles/transitions';
  import type { SitePath } from '../api';
  import ApproveRoute from '../sites/ApproveRoute.svelte';
  import type { EnrolApi } from './api';
  import { Enrolment, type ClipboardSink } from './enrol.svelte';

  interface Props {
    api: EnrolApi;
    /** The live sites: a router that connects shows here as the site's `waitingRouter`. */
    sites: readonly SitePath[];
    /** A site with no router: the modal makes a key for its router. `null`: a computer of the operator. */
    siteId: string | null;
    now?: () => Date;
    clipboard?: ClipboardSink;
    /** Runs `run` after `milliseconds`; returns the function that cancels it. */
    later?: (run: () => void, milliseconds: number) => () => void;
    onClose: () => void;
  }

  const {
    api,
    sites,
    siteId,
    now = () => new Date(),
    clipboard = navigator.clipboard,
    later = (run, milliseconds) => {
      const timer = setTimeout(run, milliseconds);
      return () => {
        clearTimeout(timer);
      };
    },
    onClose,
  }: Props = $props();

  const ids = $props.id();
  // The API, clock and clipboard are fixed for the life of the modal.
  const enrolment = untrack(() => new Enrolment(api, { now, clipboard, later }));
  let authId = $state('');

  const site = $derived(siteId === null ? undefined : sites.find((entry) => entry.site.id === siteId));
  const waiting = $derived(site?.waitingRouter ?? null);
  const view = $derived(enrolment.view);
  const key = $derived(enrolment.key);
  const notice = $derived(enrolment.expired ? 'The key expired. Make a new one.' : (enrolment.error?.message ?? ''));
  const title = $derived(
    view === 'choose' ? (siteId === null ? 'Add a computer to the VPN' : 'Add a router') : view === 'register' ? 'Register a computer' : view === 'registered' ? 'Computer registered' : siteId === null ? 'Key for a Linux computer' : 'Key for the router',
  );

  // The key is one-use: when the site's router shows up, the key has been used and leaves the page (LOG-007).
  $effect(() => {
    if (waiting !== null && enrolment.key !== null) enrolment.keyUsed();
  });

  $effect(() => {
    if (key === null) return;
    const timer = setInterval(() => {
      enrolment.tick();
    }, 1000);
    return () => {
      clearInterval(timer);
    };
  });

  /** Drops the key from this modal's state and, in the same task, from the DOM, before the modal fades (LOG-007, F-8). */
  function close(): void {
    enrolment.close();
    flushSync();
    onClose();
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    await enrolment.register(authId);
  }
</script>

<Modal open {title} describedBy="{ids}-about" closeOnScrim={key === null} onClose={close}>
  {#key view}
    <div class="body" in:fade>
      {#if view === 'choose'}
        <p id="{ids}-about">
          {siteId === null
            ? 'Pick the kind of computer. A Linux computer joins with a one-use key; a Mac or another computer joins by the ID it shows.'
            : 'Anima makes a one-use key for the site’s subnet router. The key works for one hour.'}
        </p>
        <div class="choices">
          {#if siteId === null}
            <FormButton tabindex={0} onclick={() => void enrolment.makeInvite()} aria-disabled={enrolment.busy} aria-busy={enrolment.busy}>Linux computer</FormButton>
            <FormButton tabindex={0} onclick={() => { enrolment.openRegister(); }}>Mac or other computer</FormButton>
          {:else}
            <FormButton tabindex={0} onclick={() => void enrolment.makeRouterKey(siteId)} aria-disabled={enrolment.busy} aria-busy={enrolment.busy}>
              Make a router key for {site?.site.name ?? 'the site'}
            </FormButton>
          {/if}
        </div>
        <!-- Always mounted, one line high, so a refusal moves nothing (MOT-007). -->
        <p class="notice" role="alert">{notice}</p>
      {:else if view === 'key'}
        <p id="{ids}-about">Anima shows this key only once. It works for one use. Run the steps below on the new computer.</p>
        {#if key !== null}
          <KeySecret secret={key.key} label="Pre-auth key" />
          <p class="timer" role="timer" aria-live="off">Expires in {clockText(enrolment.left)}.</p>
          <ol class="steps" aria-label="Steps">
            {#each key.steps as step (step)}
              <li>
                <code>{step}</code>
                <FormButton tabindex={0} onclick={() => void enrolment.copy(step)}>Copy</FormButton>
              </li>
            {/each}
          </ol>
          <p class="notice" role="status">
            {enrolment.copied === null ? '' : enrolment.copied.ok ? 'Copied.' : 'The step could not be copied. Select it and copy it.'}
          </p>
        {:else if waiting !== null}
          {#if site !== undefined}
            <ApproveRoute site={site.site} router={waiting} approve={api.approve} reason={null} reasonId="" />
          {/if}
        {/if}
      {:else if view === 'register'}
        <p id="{ids}-about">Type the ID from the new computer's own screen. Never use an ID from a message or a link.</p>
        <form onsubmit={submit} novalidate>
          <label for="{ids}-auth">Auth ID</label>
          <input id="{ids}-auth" type="text" autocomplete="off" autocapitalize="off" spellcheck="false" bind:value={authId} />
          <p class="notice" role="alert">{enrolment.error?.message ?? ''}</p>
          <div class="actions">
            <FormButton tabindex={0} onclick={() => { enrolment.choose(); }}>Back</FormButton>
            <FormButton tabindex={0} type="submit" tone="primary" aria-disabled={enrolment.busy || authId.trim() === ''} aria-busy={enrolment.busy}>Register</FormButton>
          </div>
        </form>
      {:else if enrolment.registered !== null}
        <p id="{ids}-about" role="status">
          {enrolment.registered.name} is on the VPN at {enrolment.registered.address}.
        </p>
      {/if}
      <div class="actions">
        <FormButton tabindex={0} onclick={close}>{view === 'registered' ? 'Done' : 'Close'}</FormButton>
      </div>
    </div>
  {/key}
</Modal>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .choices,
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .choices {
    justify-content: flex-start;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  input {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    background-color: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .timer {
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding-inline-start: var(--space-5);
  }

  .steps li {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: var(--space-2);
  }

  code {
    box-sizing: border-box;
    flex: 1 1 14rem;
    min-inline-size: 0;
    padding: var(--space-2) var(--space-3);
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    background: var(--surface-raised);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  /* One line is reserved, so a refusal or "Copied." moves nothing (MOT-007). */
  .notice {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    font-size: var(--text-sm);
  }
</style>
