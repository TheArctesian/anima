<script lang="ts">
  import type { RequestHub } from '$lib/hub/hub';
  import AccountStep from '$lib/local-hub/account/AccountStep.svelte';
  import AuthenticatorStep from '$lib/local-hub/account/AuthenticatorStep.svelte';
  import { crossFade } from '$lib/motion/roles/transitions';
  import ConnectStep from './ConnectStep.svelte';
  import { nextStep, stepHeading, type CreateEvent, type CreateStep } from './create';
  import InstallStep from './InstallStep.svelte';
  import InviteStep from './InviteStep.svelte';
  import KeyStep from './KeyStep.svelte';

  let step = $state<CreateStep>('install');
  /** The new account's name and password, from the account step until the first sign-in; then dropped (LOG-007). */
  let account = $state<{ name: string; password: string } | null>(null);
  let hub = $state<RequestHub | null>(null);
  const heading = $derived(stepHeading(step));
  /** The account steps draw their own heading, as Settings' recovery does. */
  const ownHeading = $derived(step !== 'account' && step !== 'authenticator');

  function go(event: CreateEvent): void {
    step = nextStep(step, event);
  }

  /** Each step's heading takes focus when the step shows (NAV-005); an account step focuses its own. */
  function focused(node: HTMLElement): void {
    node.focus();
  }

  function signedIn(signedInHub: RequestHub): void {
    account = null;
    hub = signedInHub;
    go({ kind: 'signed-in' });
  }
</script>

<h1>Create a hub on this computer</h1>
<p class="place" aria-live="polite">{heading.place}</p>

<!-- One step at a time; the steps cross-fade (MOT-008) and reduced motion keeps the fade only. -->
<div class="steps">
  {#key step}
    <div class="step" transition:crossFade>
      {#if ownHeading}
        <h2 tabindex="-1" {@attach focused}>{heading.title}</h2>
      {/if}
      {#if step === 'install'}
        <InstallStep onready={(accountExists) => go({ kind: 'installed', accountExists })} />
      {:else if step === 'account'}
        <AccountStep heading={heading.title} onnext={(chosen) => { account = chosen; go({ kind: 'account-chosen' }); }} />
      {:else if step === 'authenticator' && account !== null}
        <AuthenticatorStep heading={heading.title} name={account.name} password={account.password}
          onmade={() => go({ kind: 'account-made' })} onback={() => { account = null; go({ kind: 'back' }); }} />
      {:else if step === 'connect' || step === 'existing'}
        <ConnectStep account={step === 'existing' ? null : account} onsignedin={signedIn} />
      {:else if step === 'key'}
        <KeyStep ondone={() => go({ kind: 'key-done' })} />
      {:else if step === 'invite' && hub !== null}
        <InviteStep {hub} />
      {/if}
    </div>
  {/key}
</div>

<style>
  .place {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .steps {
    display: grid;
  }

  .step {
    grid-area: 1 / 1;
    display: grid;
    gap: var(--space-4);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  h2:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
