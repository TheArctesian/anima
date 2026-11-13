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
  {#if view.liveStopped}
    <p class="notice" role="alert">
      Live updates stopped. What you see was read again just now.
      <FormButton tabindex={0} onclick={() => view.resume()}>Resume live updates</FormButton>
    </p>
  {/if}
  {#if view.buildersFailed}
    <p class="notice" role="status">Anima could not read the builders; the waiting notices may be out of date.</p>
  {/if}
  <div class="columns">
    <section class="list" aria-label="Releases" aria-busy={view.releases === null && !view.loadFailed}>
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
    <div class="side">
      <section class="form" aria-label="New release">
        <h2>New release</h2>
        <ReleaseForm proposed={view.nextVersion} onrelease={(version, commit, targets) => view.release(version, commit, targets)} />
      </section>
      <div class="log">
        {#if view.selected !== null}
          <JobLog job={view.selected} lines={view.logLines} failed={view.logFailed} />
        {:else}
          <p class="quiet hint">Choose a job to read its log.</p>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-inline-size: var(--content-page);
    container-type: inline-size;
  }

  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
  }

  h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--text-lg);
  }

  .notice {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    align-items: center;
    margin: 0;
  }

  .columns {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  /* Narrow: the side's parts join the column, the form first and the log after the releases. */
  .side {
    display: contents;
  }

  .log {
    order: 1;
  }

  .form {
    order: -1;
    padding: var(--space-4);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .quiet {
    margin: 0;
    color: var(--text-secondary);
  }

  .hint {
    padding: var(--space-6) var(--space-4);
    border: var(--border-thin) dashed var(--border-strong);
    border-radius: var(--radius-md);
    text-align: center;
  }

  /* Wide: the releases on the left, the form and the log beside them, the log kept in view while the list scrolls. */
  @container (inline-size > 60rem) {
    .columns {
      display: grid;
      grid-template-columns: minmax(0, 5fr) minmax(0, 4fr);
      align-items: start;
    }

    .side {
      position: sticky;
      inset-block-start: var(--space-4);
      display: flex;
      flex-direction: column;
      gap: var(--space-6);
      min-inline-size: 0;
    }

    .log {
      --log-height: max(calc(var(--space-16) * 5), calc(100vh - var(--space-16) * 8));
    }
  }
</style>
