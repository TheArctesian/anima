<script lang="ts">
  import { saveHubKey } from '$lib/desktop/local-hub/shell';
  import FormButton from '$lib/forms/FormButton.svelte';

  const { ondone }: { ondone: () => void } = $props();
  let busy = $state(false);
  let problem = $state<string | null>(null);

  /** The shell's own save dialog writes the key; the page never sees it (2.8). */
  async function save(): Promise<void> {
    busy = true;
    problem = null;
    const saved = await saveHubKey();
    busy = false;
    if (!saved.ok) problem = saved.message;
    else if (saved.value) ondone();
  }
</script>

<p>Without the hub key the hub cannot open its data. Keep a copy with your backups, away from this computer.</p>
<p class="problem" role="alert">{problem ?? ''}</p>
<div class="actions">
  <FormButton tone="ghost" onclick={ondone}>Skip</FormButton>
  <FormButton tone="primary" {busy} onclick={() => void save()}>Save a copy</FormButton>
</div>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }

  .actions {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
  }
</style>
