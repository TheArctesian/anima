<script lang="ts">
  const CORNERS = [
    ['small', 'radius-sm, 2 px'],
    ['medium', 'radius-md, 4 px'],
    ['filled', 'radius-lg, 4 px'],
  ] as const;
</script>

<!-- The three corner radii (UR-7): 2 px for chips, 4 px for controls and surfaces. -->
<div class="corners">
  {#each CORNERS as [kind, name] (kind)}<span class="box {kind}" role="img" aria-label={name}></span>{/each}
</div>

<style>
  .corners {
    display: flex;
    gap: var(--space-4);
  }

  .box {
    inline-size: calc(var(--space-16) + var(--space-2));
    block-size: var(--space-12);
    border: var(--border-thin) solid var(--border-strong);
  }

  .small {
    border-radius: var(--radius-sm);
  }

  .medium {
    border-radius: var(--radius-md);
  }

  .filled {
    border-color: transparent;
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }
</style>
