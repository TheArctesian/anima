<script lang="ts">
  import { glyph, type IconName } from './icons';

  interface Props {
    name: IconName;
    size?: 'sm' | 'md' | 'lg';
  }

  let { name, size = 'md' }: Props = $props();
</script>

<span aria-hidden="true" class="icon icon-{size}">{glyph(name)}</span>

<style>
  .icon {
    font-family: var(--font-icon);
    line-height: 1;
  }

  .icon-sm {
    font-size: var(--icon-sm);
  }

  .icon-md {
    font-size: var(--icon-md);
  }

  .icon-lg {
    font-size: var(--icon-lg);
  }
</style>
