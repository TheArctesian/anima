<script lang="ts">
  import type { DeviceCheck } from '$lib/checks/api';
  import type { CheckFormValues } from '$lib/checks/form/model/values';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { OpenHistory } from '../detail.svelte';
  import CheckEditor from './CheckEditor.svelte';
  import CheckHistory from './CheckHistory.svelte';
  import CheckRow from './CheckRow.svelte';

  interface Props {
    checks: readonly DeviceCheck[];
    history: OpenHistory | null;
    now: () => Date;
    onHistory: (checkId: string) => void;
    onCloseHistory: () => void;
    onPause: (check: DeviceCheck, paused: boolean) => Promise<HubResult<unknown>>;
    /** `check` is null for a new one (Section 6.4). */
    onSave: (check: DeviceCheck | null, values: CheckFormValues) => Promise<HubResult<unknown>>;
    onDelete: (check: DeviceCheck) => Promise<HubResult<unknown>>;
  }

  const { checks, history, now, onHistory, onCloseHistory, onPause, onSave, onDelete }: Props = $props();

  /** The check the editor shows; kept after close so the modal's exit keeps its title. */
  let edited = $state<DeviceCheck | null>(null);
  let editing = $state(false);
  let deleting = $state<DeviceCheck | null>(null);
  const opened = $derived(checks.find((check) => check.id === history?.checkId));

  function edit(check: DeviceCheck | null): void {
    edited = check;
    editing = true;
  }

  function save(values: CheckFormValues): Promise<HubResult<unknown>> {
    return onSave(edited, values);
  }
</script>

<div class="checks">
  <button type="button" class="add" onclick={() => { edit(null); }} {@attach press()}>
    <Icon name="nf-md-plus" size="sm" />Add check
  </button>
  <ul class="rows" aria-label="Checks">
    {#each checks as check (check.id)}
      <CheckRow
        {check}
        open={history?.checkId === check.id}
        onHistory={() => {
          if (history?.checkId === check.id) onCloseHistory();
          else onHistory(check.id);
        }}
        onPause={(paused) => onPause(check, paused)}
        onEdit={() => { edit(check); }}
        onDelete={() => { deleting = check; }}
      />
    {:else}
      <li class="none">No checks yet</li>
    {/each}
  </ul>
  {#if history !== null && opened !== undefined}
    <CheckHistory checkId={opened.id} name={opened.name} pages={history.pages} {now} onClose={onCloseHistory} />
  {/if}
</div>

<CheckEditor
  open={editing}
  check={edited}
  {checks}
  onSave={save}
  onClose={() => {
    editing = false;
  }}
/>

{#if deleting !== null}
  {@const target = deleting}
  <ConfirmModal
    open
    title="Delete check"
    body="Delete {target.name}? Its heartbeat history goes too."
    confirmLabel="Delete {target.name}"
    onConfirm={async () => {
      const result = await onDelete(target);
      if (result.ok) deleting = null;
      return result;
    }}
    onCancel={() => {
      deleting = null;
    }}
  />
{/if}

<style>
  .checks {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .none {
    color: var(--text-muted);
  }

  .add {
    align-self: flex-start;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .add:hover {
    background: var(--surface-hover);
  }

  .add:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
