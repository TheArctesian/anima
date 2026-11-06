<script lang="ts">
  import type { ThisMachine } from '$lib/desktop/thisMachine';
  import { press } from '$lib/motion/roles/press';
  import { thisMachineLabel, type MachineChoice } from './this-machine';

  interface Props {
    os: ThisMachine['os'];
    chosen: MachineChoice;
    onChoose: (choice: MachineChoice) => void;
  }

  const { os, chosen, onChoose }: Props = $props();
  const options = $derived([
    { choice: 'this', label: thisMachineLabel(os) },
    { choice: 'another', label: 'Another device' },
  ] as const);
</script>

<div class="choice" role="group" aria-label="Device to add">
  {#each options as option (option.choice)}
    <button
      type="button"
      aria-pressed={chosen === option.choice}
      onclick={() => {
        onChoose(option.choice);
      }}
      {@attach press()}>{option.label}</button
    >
  {/each}
</div>

<style>
  /* The segmented look of the Grid and Table toggle on /devices. */
  .choice {
    display: flex;
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  button {
    flex: 1;
    box-sizing: border-box;
    block-size: calc(var(--target-min-desktop) + var(--space-2));
    padding: 0 var(--space-3);
    border: 0;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-raised);
  }

  button:active {
    background: var(--border);
  }

  button[aria-pressed='true'] {
    background: var(--accent-tint);
    color: var(--accent);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
