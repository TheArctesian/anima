<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';

  const tones = ['primary', 'secondary', 'ghost', 'danger'] as const;
  const sizes = ['sm', 'md', 'lg'] as const;
</script>

<section aria-labelledby="specimen-buttons">
  <h3 id="specimen-buttons">Buttons</h3>
  <div class="grid">
    {#each sizes as size (size)}
      <div class="row">
        {#each tones as tone (tone)}
          <FormButton {tone} {size}>{tone} {size}</FormButton>
        {/each}
        <FormButton tone="danger" {size} fill>danger fill {size}</FormButton>
      </div>
    {/each}
  </div>

  <h4>Leading icon and icon only</h4>
  <div class="row">
    <FormButton tone="primary" icon="nf-md-plus">Add device</FormButton>
    <FormButton icon="nf-md-cog">Settings</FormButton>
    <FormButton tone="ghost" size="sm" icon="nf-md-delete" label="Delete key" />
    <FormButton tone="secondary" icon="nf-md-cog" label="Settings" />
    <FormButton tone="danger" size="sm" icon="nf-md-delete" label="Delete device" />
  </div>

  <h4>Disabled, busy, feedback</h4>
  <div class="row">
    <FormButton tone="primary" disabled>Primary disabled</FormButton>
    <FormButton disabled>Secondary disabled</FormButton>
    <FormButton tone="ghost" disabled>Ghost disabled</FormButton>
    <FormButton tone="danger" disabled>Danger disabled</FormButton>
    <FormButton tone="primary" busy>Busy</FormButton>
  </div>
  <div class="row">
    <FormButton tone="primary" feedback="idle">Save</FormButton>
    <FormButton tone="primary" feedback="pending">Save</FormButton>
    <FormButton tone="primary" feedback="done">Save</FormButton>
    <span class="note">The three states are one width.</span>
  </div>
</section>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }

  h3,
  h4 {
    margin: 0;
  }

  .grid {
    display: grid;
    gap: var(--space-3);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .note {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
