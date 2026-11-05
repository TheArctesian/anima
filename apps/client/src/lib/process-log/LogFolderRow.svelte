<script lang="ts">
  import { openLogFolder } from '$lib/desktop/processLog';
  import { press } from '$lib/motion/roles/press';

  let problem = $state<string | null>(null);

  async function open(): Promise<void> {
    problem = null;
    try {
      await openLogFolder();
    } catch {
      problem = 'Anima could not open the log folder.';
    }
  }
</script>

<section aria-label="Logs">
  <button type="button" tabindex="0" onclick={open} {@attach press()}>Open log folder</button>
  {#if problem !== null}
    <p role="alert">{problem}</p>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    align-items: start;
    gap: var(--space-2);
    margin-block-start: var(--space-6);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
  }

  button {
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-raised);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
