<script lang="ts">
  import { iconShape, type IconName } from './icons';

  interface Props {
    name: IconName;
    size?: 'sm' | 'md' | 'lg';
  }

  let { name, size = 'md' }: Props = $props();
  const shape = $derived(iconShape(name));
</script>

<svg aria-hidden="true" class="icon icon-{size}" viewBox="0 0 {shape.width} {shape.height}" focusable="false">
  {#each shape.parts as [tag, attributes], index (index)}
    <svelte:element this={tag} {...attributes} />
  {/each}
</svg>

<style>
  .icon {
    display: inline-block;
    flex: none;
    vertical-align: -0.125em;
    fill: currentColor;
  }

  .icon-sm {
    inline-size: var(--icon-sm);
    block-size: var(--icon-sm);
  }

  .icon-md {
    inline-size: var(--icon-md);
    block-size: var(--icon-md);
  }

  .icon-lg {
    inline-size: var(--icon-lg);
    block-size: var(--icon-lg);
  }
</style>
