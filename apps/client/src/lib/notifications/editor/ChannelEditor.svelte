<script lang="ts">
  import { flushSync, untrack } from 'svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { NotificationChannel, NotificationsApi, NotificationTemplateInput } from '../api';
  import { failureText } from '../failure';
  import ChannelForm from './ChannelForm.svelte';
  import { forgetSecrets, newDraft, savedDraft, storedOf } from './draft';
  import { checkDraft, type DraftField, type DraftProblem } from './input';

  interface Props {
    api: Pick<NotificationsApi, 'create' | 'update'>;
    /** `null` adds a channel. */
    channel: NotificationChannel | null;
    /** What a channel without its own template sends (NOT-007). */
    defaultTemplate: NotificationTemplateInput;
    /** The hub's placeholders (D-21). */
    placeholders: readonly string[];
    /** The channel as the hub saved it: its target as an origin or host, no secret. */
    onSaved: (channel: NotificationChannel) => void;
    onClose: () => void;
  }

  const { api, channel, defaultTemplate, placeholders, onSaved, onClose }: Props = $props();
  // The channel and templates are read once: the editor edits the values it opened with.
  const stored = untrack(() => (channel === null ? null : storedOf(channel)));
  /** Secrets live here from the keystroke until save or close, and nowhere else (F-8, LOG-007). */
  let draft = $state(untrack(() => (channel === null ? newDraft(defaultTemplate) : savedDraft(channel, defaultTemplate))));
  let problems = $state<readonly DraftProblem[]>([]);
  let failure = $state('');
  let busy = $state(false);

  function problemOf(field: DraftField): string {
    return problems.find((problem) => problem.field === field)?.message ?? '';
  }

  /** Empties every secret from the state and, in the same task, from the fields, before the dialog fades (F-8). */
  function forget(): void {
    forgetSecrets(draft);
    flushSync();
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    const form = event.currentTarget as HTMLFormElement;
    const checked = checkDraft(draft, stored, placeholders);
    failure = '';
    problems = checked.ok ? [] : checked.problems;
    if (!checked.ok) {
      flushSync();
      form.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus();
      return;
    }
    busy = true;
    const result = channel === null ? await api.create(checked.input) : await api.update(channel.id, checked.input);
    busy = false;
    if (!result.ok) {
      failure = failureText(result.error, 'save');
      return;
    }
    forget();
    onSaved(result.value);
    onClose();
  }

  /** Refused while the hub saves, so what it answers is never lost unseen. */
  function close(): void {
    if (busy) return;
    forget();
    onClose();
  }
</script>

<Modal open title={channel === null ? 'Add a channel' : `Edit ${channel.name}`} size="form" closeOnScrim={false} onClose={close}>
  <ChannelForm bind:draft {stored} {problemOf} {failure} {busy} {placeholders} onsubmit={submit} onCancel={close} />
</Modal>
