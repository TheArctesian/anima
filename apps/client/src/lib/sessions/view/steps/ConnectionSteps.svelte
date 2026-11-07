<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { connectionSteps, wakeLine, type Progress, type StepMark } from './steps';

  interface Props {
    progress: Progress;
    /** Shown as text in the wake line. */
    deviceName: string;
  }

  const { progress, deviceName }: Props = $props();
  const steps = $derived(connectionSteps(progress));
  const wake = $derived(wakeLine(progress, deviceName));

  const MARK_ICON: Readonly<Record<StepMark, IconName>> = {
    waiting: 'nf-md-dots_vertical',
    active: 'nf-md-refresh',
    done: 'nf-md-check_circle',
    failed: 'nf-md-close_circle',
  };

  const MARK_TEXT: Readonly<Record<StepMark, string>> = {
    waiting: 'waiting',
    active: 'in progress',
    done: 'done',
    failed: 'stopped',
  };
</script>

<!-- Each mark changes when its step ends, with no easing and no transition (Section 5.7.3 "Connection steps"). -->
<ol class="steps" aria-label="Connection steps">
  {#each steps as step (step.name)}
    <li class="step" data-step={step.name} data-mark={step.mark} aria-current={step.mark === 'active' ? 'step' : undefined}>
      <Icon name={MARK_ICON[step.mark]} size="sm" />
      <span class="label">{step.label}</span>
      <span class="mark">{MARK_TEXT[step.mark]}</span>
    </li>
  {/each}
</ol>
<!--
  The slot is always there, one line high, so the line filling it moves nothing (MOT-007, MV-004); its mark changes
  with no easing (MOT-009). The mark word is read, not shown.
-->
<p class="wake" data-wake={wake?.mark}>
  {#if wake !== undefined}
    <Icon name={MARK_ICON[wake.mark]} size="sm" />
    <span class="label">{wake.label}</span>
    <span class="sr">{MARK_TEXT[wake.mark]}</span>
  {/if}
</p>

<style>
  /*
   * Equal, fixed columns, each mark word on its own line: a mark that changes its word moves no other step
   * (MV-004, MOT-007).
   */
  .steps {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, calc(var(--space-16) * 2)));
    gap: var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* A fixed icon cell: the mark glyphs differ in width, and an `auto` column would move the label (MOT-007). */
  .step {
    display: grid;
    grid-template-columns: var(--icon-md) 1fr;
    column-gap: var(--space-1);
    align-items: center;
    color: var(--text-muted);
  }

  .step > :global(.icon) {
    justify-self: center;
  }

  /* One line: a long device name in the waking step is cut, so the row keeps its height (MOT-007). */
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .wake {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-inline-size: 0;
    max-inline-size: 100%;
    block-size: 1lh;
    margin: var(--space-2) 0 0;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
    color: var(--text);
  }

  .wake > :global(.icon) {
    flex: none;
  }

  .wake[data-wake='done'] :global(.icon) {
    color: var(--status-up);
  }

  .wake[data-wake='failed'] :global(.icon) {
    color: var(--status-down);
  }

  .sr {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .mark {
    grid-column: 2;
    color: var(--text-muted);
  }

  .step[data-mark='active'],
  .step[data-mark='done'] {
    color: var(--text);
  }

  .step[data-mark='done'] :global(.icon) {
    color: var(--status-up);
  }

  .step[data-mark='failed'] :global(.icon) {
    color: var(--status-down);
  }
</style>
