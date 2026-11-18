<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { STEPS, stepStates, type StepId } from './steps';

  /** The step that runs now, whether it failed, and a short line under a step (the device key under "Link started"). */
  const { current, failed, notes }: { current: StepId; failed: boolean; notes: Partial<Record<StepId, string>> } = $props();
  const states = $derived(stepStates(current, failed));
</script>

<!-- Frame 2 of docs/mockup/mobile/index.html, "Joining": one row for each step, its mark changes when the step ends. -->
<ol class="steps">
  {#each STEPS as step (step.id)}
    <li class={states[step.id]} aria-current={states[step.id] === 'now' ? 'step' : undefined}>
      <span class="mark" aria-hidden="true">
        {#if states[step.id] === 'done'}<Icon name="carbon:checkmark" size="sm" />{/if}
        {#if states[step.id] === 'failed'}<Icon name="carbon:close" size="sm" />{/if}
      </span>
      <span>
        {step.title}
        {#if notes[step.id] !== undefined}<small>{notes[step.id]}</small>{/if}
      </span>
    </li>
  {/each}
</ol>

<style>
  .steps {
    display: grid;
    margin: var(--space-4) 0 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    gap: calc(var(--space-3) + var(--border-focus));
    align-items: center;
    min-block-size: calc(var(--space-12) + var(--space-2));
    border-block-end: var(--border-thin) solid var(--border-subtle);
    font-size: var(--text-phone-row, var(--text-md));
  }

  small {
    display: block;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .mark {
    display: grid;
    flex: none;
    place-items: center;
    box-sizing: border-box;
    inline-size: var(--space-6);
    block-size: var(--space-6);
    border: var(--border-focus) solid var(--border-strong);
    border-radius: 50%;
    color: var(--on-accent);
  }

  .done .mark {
    border-color: var(--status-up);
    background: var(--status-up);
  }

  .failed .mark {
    border-color: var(--status-down);
    background: var(--status-down);
  }

  .now .mark {
    border-color: var(--accent);
    border-inline-end-color: transparent;
  }

  .wait {
    color: var(--text-tertiary);
  }
</style>
