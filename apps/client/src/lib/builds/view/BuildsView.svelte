<script lang="ts">
  import { useHub } from '$lib/hub/context';
  import FormButton from '$lib/forms/FormButton.svelte';
  import JobLog from './JobLog.svelte';
  import ReleaseForm from './ReleaseForm.svelte';
  import ReleaseList from './ReleaseList.svelte';
  import { ReleaseBoard, REFRESH_MS } from './board.svelte';

  const view = new ReleaseBoard(useHub());

  $effect(() => {
    void view.open();
    const timer = setInterval(() => void view.tick(), REFRESH_MS);
    return () => {
      clearInterval(timer);
      view.stop();
    };
  });
</script>

<div class="page">
  <h1>Builds</h1>
  <section aria-label="New release">
    <ReleaseForm proposed={view.nextVersion} onrelease={(version, commit, targets) => view.release(version, commit, targets)} />
  </section>
  {#if view.liveStopped}
    <p role="alert">
      Live updates stopped. What you see was read again just now.
      <FormButton tabindex={0} onclick={() => view.resume()}>Resume live updates</FormButton>
    </p>
  {/if}
  {#if view.buildersFailed}
    <p role="status">Anima could not read the builders; the waiting notices may be out of date.</p>
  {/if}
  <section aria-label="Releases" aria-busy={view.releases === null && !view.loadFailed}>
    {#if view.loadFailed && view.releases === null}
      <p role="alert">Anima could not load the releases.</p>
      <FormButton tabindex={0} onclick={() => view.loadReleases()}>Try again</FormButton>
    {:else if view.releases !== null && view.releases.length === 0}
      <p class="quiet">No releases yet.</p>
    {:else if view.releases !== null}
      <ReleaseList
        releases={view.ordered}
        builders={view.builders}
        now={view.now}
        selectedId={view.selectedId}
        cancelError={view.cancelError}
        onselect={(job) => view.select(job)}
        oncancel={(job) => view.cancel(job)}
      />
    {/if}
  </section>
  {#if view.selected !== null}
    <JobLog job={view.selected} lines={view.logLines} failed={view.logFailed} />
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-inline-size: calc(var(--space-16) * 11);
  }

  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
  }

  .quiet {
    color: var(--text-secondary);
  }
</style>
