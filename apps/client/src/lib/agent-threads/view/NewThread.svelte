<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { EFFORTS, MODE_WORDS, offeredCopy, startOf, type NewThreadForm } from '../live/drive';
  import type { CallResult } from '../live/live.svelte';
  import type { Action, Effort, Mode } from '../live/run';
  import type { Program } from '../thread';

  interface Props {
    open: boolean;
    onStart: (action: Action) => Promise<CallResult>;
    /** The thread started: its ID. */
    onStarted: (thread: string) => void;
    onClose: () => void;
  }

  const { open, onStart, onStarted, onClose }: Props = $props();
  let program = $state<Program>('claude_code');
  let folder = $state('');
  let mode = $state<Mode>('full_access');
  let model = $state('');
  let effort = $state<Effort | ''>('');
  let prompt = $state('');
  /** The copy of the program, empty for the usual one (TH97). */
  let programPath = $state('');
  /** A copy that answered when the chosen one did not: the dialog offers it. */
  let offer = $state<string | null>(null);
  let busy = $state(false);
  let problem = $state<string | null>(null);
  /** The folder whose notice was read, and the notice: what its project files make the program do (THR-046). */
  let read = $state<string | null>(null);
  let notice = $state<readonly string[]>([]);
  const MODES = Object.keys(MODE_WORDS) as Mode[];
  const NOT_CHECKED =
    'The project files of this folder could not be checked: they can make Claude Code run commands with no question.';

  async function start(): Promise<void> {
    if (busy) return;
    const form: NewThreadForm = { program, folder, mode, model, effort, prompt, programPath };
    const checked = startOf(form);
    if (!checked.ok) {
      problem = checked.words;
      return;
    }
    busy = true;
    problem = null;
    offer = null;
    // The first press reads the folder; when its files run something by themselves, the notice is shown first and the
    // thread starts at the next press (OD-THR21: project settings are honoured, with a notice).
    if (read !== folder && !(await readNotice())) {
      busy = false;
      return;
    }
    const result = await onStart(checked.action);
    busy = false;
    if (!result.ok) {
      // The files can change before the next press: it reads them again.
      read = null;
      problem = result.words;
      offer = offeredCopy(result, programPath);
      return;
    }
    if (result.outcome.outcome === 'started') onStarted(result.outcome.thread);
    prompt = '';
    onClose();
  }

  /** Starts again with the copy that answered. */
  function useOffer(copy: string): void {
    if (busy) return;
    programPath = copy;
    offer = null;
    void start();
  }

  /** Reads the notice of the folder; true when the start may go on now. A check that fails is shown as a notice and
   *  does not stop the start: the next press starts. */
  async function readNotice(): Promise<boolean> {
    const asked = folder;
    const result = await onStart({ call: 'folder', folder: asked.trim() });
    read = asked;
    if (!result.ok) {
      notice = [NOT_CHECKED];
      return false;
    }
    notice = result.outcome.outcome === 'folder' ? result.outcome.notice : [];
    return notice.length === 0;
  }
</script>

<!-- A new thread (THR-020): the program, the folder of this computer, the mode of approvals (full access is the
     default) and the first prompt. The runner checks each value again. -->
<Modal {open} title="New thread" {onClose} preset="md" dismissible>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      void start();
    }}
  >
    <Field label="Program">
      <Select
        bind:value={program}
        onchange={() => {
          // A copy belongs to one program.
          programPath = '';
          offer = null;
        }}
      >
        <option value="claude_code">Claude Code</option>
        <option value="codex">Codex</option>
      </Select>
    </Field>
    <Field label="Folder" constraint="An absolute folder of this computer, for example /Users/you/project">
      <Input bind:value={folder} mono placeholder="/Users/you/project" autocomplete="off" spellcheck={false} />
    </Field>
    <Field label="Approvals">
      <Select bind:value={mode}>
        {#each MODES as each (each)}<option value={each}>{MODE_WORDS[each]}</option>{/each}
      </Select>
    </Field>
    <Field label="Model" constraint="Empty for the default model of the program">
      <Input bind:value={model} mono autocomplete="off" spellcheck={false} />
    </Field>
    <Field label="Effort">
      <Select bind:value={effort}>
        <option value="">Default</option>
        {#each EFFORTS as each (each)}<option value={each}>{each}</option>{/each}
      </Select>
    </Field>
    <Field label="First prompt">
      <textarea class="prompt" bind:value={prompt} rows="4" aria-label="First prompt"></textarea>
    </Field>
    <details class="advanced" open={programPath !== ''}>
      <summary>Advanced</summary>
      <Field label="Program path" constraint="Empty for the usual copy. Only a copy that the runner finds is taken">
        <!-- A path typed by hand replaces the offer: the button would overwrite it. -->
        <Input bind:value={() => programPath, (next) => { programPath = next; offer = null; }} mono placeholder="/Applications/…/codex" autocomplete="off" spellcheck={false} />
      </Field>
    </details>
    {#if read === folder && notice.length > 0}
      <div class="notice" role="note" aria-label="What this folder runs by itself">
        <p>{notice[0]}</p>
        <ul>
          {#each notice.slice(1) as line, index (index)}<li>{line}</li>{/each}
        </ul>
        <p>Press Start again to start the thread with these settings.</p>
      </div>
    {/if}
    {#if offer !== null}
      <div class="offer" role="alert">
        <p>{programPath === '' ? 'The usual copy' : 'The chosen copy'} of {program === 'codex' ? 'Codex' : 'Claude Code'} does not answer. A different copy answers:</p>
        <p class="copy">{offer}</p>
        <FormButton type="button" tone="primary" busy={busy} onclick={() => useOffer(offer ?? '')}>Start with this copy</FormButton>
      </div>
    {:else if problem !== null}<p class="problem" role="alert">{problem}</p>{/if}
    <div class="acts">
      <FormButton type="button" onclick={onClose}>Cancel</FormButton>
      <FormButton type="submit" tone="primary" busy={busy}>Start</FormButton>
    </div>
  </form>
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .prompt {
    inline-size: 100%;
    padding: var(--space-2);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text-primary);
    font: inherit;
    resize: vertical;
  }

  .acts {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .notice {
    padding: var(--space-3);
    border: var(--border-thin) solid var(--status-degraded);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    overflow-wrap: anywhere;
  }

  .notice p,
  .notice ul {
    margin: 0;
  }

  .notice ul {
    margin-block: var(--space-2);
    padding-inline-start: var(--space-4);
  }

  .advanced summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .offer {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-thin) solid var(--status-degraded);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .offer p {
    margin: 0;
  }

  .offer .copy {
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }

  .problem {
    margin: 0;
    color: var(--status-down);
    font-size: var(--text-sm);
  }
</style>
