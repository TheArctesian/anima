<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { connectionSteps, type Progress, type StepMark } from './steps';

  interface Props {
    progress: Progress;
  }

  const { progress }: Props = $props();
  const steps = $derived(connectionSteps(progress));

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
      <span>{step.label}</span>
      <span class="mark">{MARK_TEXT[step.mark]}</span>
    </li>
  {/each}
</ol>

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

  .step {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: var(--space-1);
    align-items: center;
    color: var(--text-muted);
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
