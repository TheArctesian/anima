<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { Stage, StageMark } from './progress';

  interface Props {
    stages: readonly Stage[];
  }

  const { stages }: Props = $props();

  /** What a screen reader hears after the stage's name. */
  const SPOKEN: Record<StageMark, string> = {
    done: 'done',
    current: 'in progress',
    failed: 'failed',
    pending: 'not started',
    skipped: 'not run',
  };
</script>

<!-- Each mark changes when its stage ends, with no easing and no repeating motion (5.7.3 Connection steps, MOT-009, MOT-011). -->
<ol class="track" aria-label="Stages">
  {#each stages as stage (stage.name)}
    <li class="stage {stage.mark}">
      <span class="mark" aria-hidden="true">
        {#if stage.mark === 'done'}<Icon name="nf-md-check" size="sm" />
        {:else if stage.mark === 'failed'}<Icon name="nf-md-close" size="sm" />
        {/if}
      </span>
      <span class="words">
        <span class="label">{stage.label}<span class="spoken">: {SPOKEN[stage.mark]}</span></span>
        {#if stage.detail !== '' && (stage.mark === 'current' || stage.mark === 'failed')}
          <span class="detail">{stage.detail}</span>
        {/if}
      </span>
    </li>
  {/each}
</ol>

<style>
  .track {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .stage {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-small);
  }

  /* The rail between two marks: done stages join with the success colour, the rest stay quiet. */
  .stage:not(:last-child)::after {
    position: absolute;
    inset-block-start: var(--space-2);
    inset-inline: calc(var(--space-4) + var(--space-1)) var(--space-1);
    block-size: var(--border-thin);
    background: var(--border-subtle);
    content: '';
  }

  .stage.done:not(:last-child)::after {
    background: var(--status-up);
  }

  .mark {
    display: inline-grid;
    flex: none;
    place-items: center;
    inline-size: var(--space-4);
    block-size: var(--space-4);
    box-sizing: border-box;
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    font-size: var(--text-xs);
  }

  .words {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    padding-inline-end: var(--space-2);
  }

  .label {
    font-weight: var(--weight-medium);
  }

  .detail {
    overflow-wrap: anywhere;
  }

  .done {
    color: var(--text-secondary);
  }

  .done .mark {
    border-color: var(--status-up);
    background: var(--status-up-tint);
    color: var(--status-up-text);
  }

  .current {
    color: var(--text);
  }

  .current .mark {
    border: var(--border-focus) solid var(--accent);
    background: var(--accent-tint);
  }

  .failed {
    color: var(--status-down-text);
  }

  .failed .mark {
    border-color: var(--status-down);
    background: var(--status-down-tint);
    color: var(--status-down-text);
  }

  .skipped .mark {
    border-style: dashed;
    background: transparent;
  }

  /* Read by a screen reader, not drawn: the mark carries it visually. */
  .spoken {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
