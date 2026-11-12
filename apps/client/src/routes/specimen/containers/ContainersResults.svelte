<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import Section from '$lib/page/Section.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';

  let tries = $state(0);
</script>

<div class="stack">
  <h3>Empty, loading and error states, each sized to its message</h3>
  <div class="trio">
    <Section title="Empty">
      <EmptyState title="No devices yet" line="Add your first device to start watching it.">
        {#snippet action()}<FormButton tone="primary">Add device</FormButton>{/snippet}
      </EmptyState>
    </Section>
    <Section title="Loading">
      <LoadingState label="Loading devices" rows={3} />
    </Section>
    <Section title="Error">
      <ErrorState message="Devices did not load." onRetry={() => (tries += 1)}>
        {#snippet cause()}<span class="tip" title="503 Service Unavailable from the hub">?</span>{/snippet}
      </ErrorState>
      <p class="tries">Tried again {tries} times.</p>
    </Section>
  </div>

  <h3>Notices: info, success, warning, error</h3>
  <Section title="Notification settings">
    <div class="notices">
      <Notice tone="info" title="Updates are checked daily" line="The next check runs tonight at 03:00." />
      <Notice tone="success" title="Saved" line="Your changes are in effect." />
      <Notice tone="warning" title="The certificate expires soon" line="It expires in 6 days; renew it before then." />
      <Notice tone="error" title="The test failed" line="The hub could not reach smtp.example.org on port 587." />
    </div>
  </Section>
</div>

<style>
  .stack {
    display: grid;
    gap: var(--space-3);
  }

  h3 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  .trio {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
    align-items: start;
    gap: var(--space-4);
  }

  .notices {
    display: grid;
    gap: var(--space-2);
  }

  .tries {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    text-align: center;
  }

  .tip {
    display: inline-grid;
    place-items: center;
    inline-size: var(--space-4);
    block-size: var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }
</style>
