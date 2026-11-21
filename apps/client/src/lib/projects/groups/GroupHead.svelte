<script lang="ts">
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { ProjectGroups } from './groups.svelte';
  import NameModal from './NameModal.svelte';
  import type { GroupSection } from './view';

  interface Props {
    groups: ProjectGroups;
    section: GroupSection;
    open: boolean;
    onToggle: () => void;
    /** The id of the list that this head opens and closes. */
    controls: string;
  }

  const { groups, section, open, onToggle, controls }: Props = $props();
  let renaming = $state(false);
  let deleting = $state(false);
  const id = $derived(section.id);
</script>

<!-- The head of one group in the list shown by group (PRO-061): its name and count open and close it; a group that the
     operator made has Rename and Delete at its end (PRO-062). -->
<div class="head">
  <h2>
    <button type="button" class="toggle" aria-expanded={open} aria-controls={controls} onclick={onToggle}>
      <span class="chevron" class:open><Icon name="carbon:chevron-right" size="sm" /></span>{section.title}<span class="count"
        >{section.projects.length}</span
      >
    </button>
  </h2>
  {#if id !== null}
    <span class="actions">
      <FormButton tone="ghost" size="sm" onclick={() => (renaming = true)}>Rename</FormButton>
      <FormButton tone="ghost" size="sm" onclick={() => (deleting = true)}>Delete</FormButton>
    </span>
    <NameModal
      open={renaming}
      title="Rename group"
      start={section.title}
      saveLabel="Rename"
      onSave={(name) => groups.rename(id, name)}
      onClose={() => (renaming = false)}
    />
    {#if deleting}
      <ConfirmModal
        open={deleting}
        title={`Delete the group ${section.title}?`}
        body="The group goes from this screen. Its cards stay, under Ungrouped: no project changes."
        confirmLabel="Delete group"
        onConfirm={() => {
          groups.delete(id);
          return Promise.resolve({ ok: true, value: null });
        }}
        onCancel={() => (deleting = false)}
      />
    {/if}
  {/if}
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-inline-size: 0;
  }

  h2 {
    margin: 0;
    min-inline-size: 0;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    max-inline-size: 100%;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
    cursor: pointer;
  }

  .toggle:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  .chevron {
    display: inline-flex;
    color: var(--text-tertiary);
    transition: rotate var(--duration-fast) var(--ease-enter);
  }

  .chevron.open {
    rotate: 90deg;
  }

  .count {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: inline-flex;
    gap: var(--space-1);
    margin-inline-start: auto;
  }
</style>
