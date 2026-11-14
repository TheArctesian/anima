<script lang="ts">
  import { flushSync } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import { duration } from '$lib/motion/tokens';
  import type { GithubAppInput } from '../api';
  import { checkConnect, EMPTY_FORM, KEY_FILE_UNREADABLE, keyFileProblem, restorePem, type ConnectField, type ConnectProblem } from './connect';

  interface Props {
    /** Sends the connection; the dialog keeps no copy of what it passes (PRJ-018). */
    onConnect: (input: GithubAppInput) => Promise<HubResult<unknown>>;
    onClose: () => void;
  }

  const { onConnect, onClose }: Props = $props();
  const ids = $props.id();
  const form = $state({ ...EMPTY_FORM });
  let problems = $state<readonly ConnectProblem[]>([]);
  let failure = $state('');
  let busy = $state(false);
  let fileField: HTMLInputElement | undefined = $state();

  const problemOf = (field: ConnectField): string => problems.find((problem) => problem.field === field)?.message ?? '';

  /**
   * Empties the field the moment it starts to leave. A closing dialog keeps its last content on screen until the fade
   * ends; the key must not stay in the DOM for that time (F-8).
   */
  function drop(node: HTMLInputElement): { duration: number } {
    node.value = '';
    return { duration: duration.instant };
  }

  /** The key, and the chosen file, are emptied from the state and, in the same task, from the page. */
  function forgetKey(): void {
    form.privateKey = '';
    if (fileField !== undefined) fileField.value = '';
    flushSync();
  }

  /** Reads a chosen `.pem` file in the browser into the key field; the file goes nowhere else. */
  async function choose(event: Event & { currentTarget: HTMLInputElement }): Promise<void> {
    const file = event.currentTarget.files?.[0];
    if (file === undefined) return;
    const tooLarge = keyFileProblem(file);
    try {
      if (tooLarge !== null) {
        problems = [{ field: 'privateKey', message: tooLarge }];
        return;
      }
      form.privateKey = restorePem(await file.text());
      problems = problems.filter((problem) => problem.field !== 'privateKey');
    } catch {
      // The reason is not shown: the file's content is a key.
      problems = [{ field: 'privateKey', message: KEY_FILE_UNREADABLE }];
    } finally {
      event.currentTarget.value = '';
    }
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    const checked = checkConnect(form);
    failure = '';
    if (!checked.ok) {
      problems = checked.problems;
      document.getElementById(`${ids}-${checked.problems[0]?.field ?? 'label'}`)?.focus();
      return;
    }
    problems = [];
    busy = true;
    // The key is on its way: the dialog holds none, whatever the hub answers.
    forgetKey();
    const result = await onConnect(checked.input);
    busy = false;
    if (result.ok) onClose();
    else failure = result.error.message;
  }

  function close(): void {
    if (busy) return;
    forgetKey();
    onClose();
  }
</script>

<Modal open title="Connect GitHub" describedBy="{ids}-about" width={7} onClose={close}>
  <div class="body">
    <p id="{ids}-about">
      You make the GitHub App in your own account (GitHub, Settings, Developer settings, GitHub Apps) and install it. Anima does not
      make it. Enter its App ID, the installation ID and the private key. The hub seals the key, and Anima does not show it again.
    </p>
    <!-- No `name` attributes: a form that were ever submitted by the browser would put its fields in the URL (F-8). -->
    <form onsubmit={submit} novalidate>
      <fieldset class="fields" disabled={busy}>
        <div class="field">
          <label for="{ids}-label">Name</label>
          <input id="{ids}-label" type="text" autocomplete="off" spellcheck="false" bind:value={form.label} aria-invalid={problemOf('label') !== ''} />
          <small class="problem">{problemOf('label')}</small>
        </div>
        <div class="field">
          <label for="{ids}-appId">App ID</label>
          <input id="{ids}-appId" type="text" inputmode="numeric" autocomplete="off" spellcheck="false" bind:value={form.appId} aria-invalid={problemOf('appId') !== ''} />
          <small class="problem">{problemOf('appId')}</small>
        </div>
        <div class="field">
          <label for="{ids}-installationId">Installation ID</label>
          <input id="{ids}-installationId" type="text" inputmode="numeric" autocomplete="off" spellcheck="false" bind:value={form.installationId} aria-invalid={problemOf('installationId') !== ''} />
          <small class="problem">{problemOf('installationId')}</small>
        </div>
        <div class="field">
          <label for="{ids}-privateKey">Private key</label>
          <input
            id="{ids}-privateKey"
            out:drop|global
            type="password"
            autocomplete="new-password"
            autocapitalize="off"
            spellcheck="false"
            bind:value={form.privateKey}
            aria-invalid={problemOf('privateKey') !== ''}
            aria-describedby="{ids}-key-hint"
          />
          <small id="{ids}-key-hint">Paste the whole key, or choose the .pem file from GitHub below. The field hides it.</small>
          <input bind:this={fileField} type="file" accept=".pem,application/x-pem-file" aria-label="Choose the .pem file" onchange={choose} />
          <small class="problem">{problemOf('privateKey')}</small>
        </div>
      </fieldset>
      <p class="failure" role="alert">{failure}</p>
      <div class="actions">
        <FormButton tabindex={0} onclick={close} aria-disabled={busy}>Cancel</FormButton>
        <FormButton tabindex={0} type="submit" tone="primary" aria-disabled={busy} aria-busy={busy}>Connect</FormButton>
      </div>
    </form>
  </div>
</Modal>

<style>
  .body,
  form,
  .fields,
  .field {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }

  .body,
  form {
    gap: var(--space-3);
  }

  .fields {
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    border: 0;
  }

  .field {
    gap: var(--space-1);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  label,
  small {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  small {
    font-size: var(--text-xs);
  }

  .problem:not(:empty),
  .failure:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
    color: var(--text);
  }

  .problem:empty {
    display: none;
  }

  input[type='text'],
  input[type='password'] {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background-color: var(--surface-raised);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  [aria-invalid='true'] {
    border-color: var(--status-down);
  }

  .failure {
    min-block-size: 1lh;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
