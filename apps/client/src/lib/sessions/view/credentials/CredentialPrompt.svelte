<script lang="ts">
  import { press } from '$lib/motion/roles/press';
  import { fade, modal } from '$lib/motion/roles/transitions';
  import type { RemoteProtocol, SessionsApi } from '../../api';
  import { trapFocus } from '../hostkey/focus';
  import type { ArgumentClient } from '../../tunnel/argv';
  import { sendArguments, type Answers } from './arguments';
  import type { CredentialField, CredentialName } from './fields';

  interface Props {
    device: { readonly id: string; readonly name: string };
    protocol: RemoteProtocol;
    /** What to ask, from `askedFields`. */
    fields: readonly CredentialField[];
    client: ArgumentClient;
    api: Pick<SessionsApi, 'setCredential'>;
    /** The values were sent to the gateway; close the prompt and let the session go on. */
    onsent: () => void;
    /** The operator declined: nothing was sent, and the caller ends the session. */
    oncancel: () => void;
  }

  const { device, protocol, fields, client, api, onsent, oncancel }: Props = $props();

  /** Fixed text; the hub's own message is never shown (R-035, LOG-007). */
  const UNSAVED = 'The hub did not save the password. It was used for this session only, so Anima will ask again next time.';
  const EMPTY: Record<CredentialName, string> = { username: '', domain: '', password: '' };

  let values: Record<CredentialName, string> = $state({ ...EMPTY });
  let save = $state(false);
  let step: 'asking' | 'saving' | 'sent' | 'unsaved' = $state('asking');
  let surface: HTMLElement | undefined = $state();
  let closeButton: HTMLButtonElement | undefined = $state();
  let statusLine: HTMLElement | undefined = $state();
  const offersSave = $derived(fields.some((field) => field.secret));

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (step !== 'asking') return;
    const answers: Answers = { ...values };
    const password = save && offersSave && answers.password !== '' ? answers.password : undefined;
    // The prompt holds nothing once the values are on their way.
    values = { ...EMPTY };
    save = false;
    sendArguments(client, fields, answers);
    if (password !== undefined) {
      step = 'saving';
      const result = await api.setCredential(protocol, device.id, password);
      if (!result.ok || !result.value.credentialStored) {
        step = 'unsaved';
        return;
      }
    }
    step = 'sent';
    onsent();
  }

  function escape() {
    if (step === 'asking') oncancel();
    else if (step === 'unsaved') onsent();
  }

  // After Submit the form is disabled, so focus moves to what the prompt says now, never to the page behind it.
  $effect(() => {
    if (step === 'asking' || surface === undefined) return;
    (step === 'unsaved' ? closeButton : statusLine)?.focus();
  });
</script>

<div class="layer">
  <div class="scrim" transition:fade><div class="shade"></div></div>
  <div
    class="surface"
    role="dialog"
    aria-modal="true"
    aria-labelledby="credential-title"
    aria-describedby="credential-why"
    bind:this={surface}
    transition:modal
    {@attach trapFocus(escape)}
  >
    <h2 id="credential-title">Sign in to {device.name}</h2>
    <p id="credential-why">
      {device.name} asks for its {protocol} sign-in. Anima sends what you type to this session only; the hub keeps the
      password only when you choose "Save password".
    </p>

    {#if step !== 'unsaved'}
      <!-- Disabled, and empty, once sent: what was typed is gone from the prompt (GW-015). -->
      <form onsubmit={submit}>
        <fieldset disabled={step !== 'asking'}>
          {#each fields as field (field.name)}
            <label>
              <span>{field.label}</span>
              <!--
                autocomplete="off": these are the device's credentials, not Anima's. A password manager would file them
                under the hub's address and offer them for every device; the only place Anima keeps one is the hub,
                and only on "Save password" (GW-015).
              -->
              {#if field.secret}
                <input type="password" name={field.name} autocomplete="off" bind:value={values[field.name]} />
              {:else}
                <input
                  type="text"
                  name={field.name}
                  autocomplete="off"
                  autocapitalize="off"
                  spellcheck="false"
                  bind:value={values[field.name]}
                />
              {/if}
            </label>
          {/each}
          {#if offersSave}
            <label class="save"><input type="checkbox" bind:checked={save} /> Save password</label>
          {/if}
          <div class="actions">
            <button type="button" onclick={oncancel} {@attach press()}>Cancel</button>
            <button type="submit" class="primary" {@attach press()}>Sign in</button>
          </div>
        </fieldset>
      </form>
      {#if step !== 'asking'}
        <p class="status" role="status" tabindex="-1" bind:this={statusLine}>
          {step === 'saving' ? 'Saving the password…' : 'Signing in…'}
        </p>
      {/if}
    {:else}
      <p class="problem" role="status">{UNSAVED}</p>
      <div class="actions">
        <button type="button" bind:this={closeButton} onclick={onsent} {@attach press()}>Close</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .layer {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: start center;
    z-index: var(--layer-modal);
  }

  .scrim,
  .surface {
    grid-area: 1 / 1;
  }

  .scrim {
    place-self: stretch;
  }

  .shade {
    block-size: 100%;
    background: var(--scrim);
  }

  .surface {
    position: relative;
    box-sizing: border-box;
    inline-size: min(100% - var(--space-8), calc(var(--space-16) * 7));
    max-block-size: calc(100vh - var(--space-16) * 2);
    margin-block-start: var(--space-16);
    overflow-y: auto;
    padding: var(--space-6);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--elevation-3);
    color: var(--text);
    font-size: var(--text-md);
    line-height: var(--leading-normal);
  }

  h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--text-lg);
    line-height: var(--leading-tight);
  }

  p {
    margin: 0 0 var(--space-3);
  }

  fieldset {
    display: grid;
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    border: 0;
  }

  .status {
    margin: var(--space-3) 0 0;
  }

  .status:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  label {
    display: grid;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }

  .save {
    display: flex;
    min-block-size: var(--target-min-desktop);
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-md);
  }

  input:not([type='checkbox']) {
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
  }

  .problem {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--status-down-tint);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  button {
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  input:focus-visible,
  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
  }
</style>
