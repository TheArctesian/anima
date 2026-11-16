<script lang="ts">
  import type { Snippet } from 'svelte';
  import { asset } from '$app/paths';

  const { children }: { children: Snippet } = $props();
</script>

<!-- The first run in the sign-in page's language (UR-7): one column on the bare ground, no bar (NAV-009, FRN-001). -->
<div class="screen">
  <div class="panel">
    <p class="label"><img src={asset('/logo.png')} alt="" width="22" height="22" /><span class="rule" aria-hidden="true"></span>Anima.ing</p>
    {@render children()}
  </div>
</div>

<style>
  /* Pinned to the top, not centred: a step that grows or shrinks moves nothing above it (MOT-007). */
  .screen {
    box-sizing: border-box;
    display: grid;
    justify-items: center;
    align-items: start;
    padding: var(--space-16) var(--space-4) var(--space-6);
  }

  .panel {
    box-sizing: border-box;
    display: grid;
    gap: var(--space-4);
    inline-size: min(100%, calc(var(--space-16) * 8));
  }

  .label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .rule {
    inline-size: var(--space-8);
    block-size: var(--border-thin);
    background: var(--border-strong);
  }

  /* A field's error has its line before it shows, so the form below does not move when it does (MOT-007). */
  .panel :global(.field [role='alert']) {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
  }

  .panel :global(h1) {
    margin: 0 0 var(--space-2);
    font-size: var(--text-display);
    font-weight: var(--weight-light);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
  }
</style>
