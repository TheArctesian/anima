<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { flip, measure } from '$lib/motion/roles/flip';

  interface Props {
    /** `general`, `ssh` or `rdp`: how the form finds the section (DEV-022). */
    name: string;
    title: string;
    open: boolean;
    children: Snippet;
  }

  let { name, title, open = $bindable(), children }: Props = $props();
  const bodyId = $props.id();
  let section = $state<HTMLElement>();

  /** Disclosure role (5.7.3): the sections below move to their new place on `spring-default`; the content fades. */
  async function toggle(): Promise<void> {
    const sections = section?.parentElement?.querySelectorAll<HTMLElement>('[data-disclosure]') ?? [];
    const first = measure(sections);
    open = !open;
    await tick();
    flip(first, { spring: 'default' });
  }
</script>

<section data-disclosure data-section={name} bind:this={section}>
  <h3>
    <button type="button" aria-expanded={open} aria-controls={bodyId} onclick={toggle}>
      <Icon name={open ? 'nf-md-chevron_down' : 'nf-md-chevron_right'} size="sm" />
      {title}
    </button>
  </h3>
  {#if open}
    <div id={bodyId} class="body" transition:crossFade>
      {@render children()}
    </div>
  {/if}
</section>

<style>
  section {
    border-top: var(--border-thin) solid var(--border);
    padding-block: var(--space-2);
  }

  h3 {
    margin: 0;
    font-size: var(--text-md);
  }

  h3 button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    inline-size: 100%;
    min-block-size: var(--target-min-desktop);
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    font: inherit;
    font-weight: var(--weight-bold);
    text-align: start;
  }

  h3 button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-block: var(--space-2);
  }
</style>
