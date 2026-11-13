<script lang="ts">
  /** The three materials (5.6, MAT-010) over a busy backdrop: stripes, text and colour, so the blur shows. */
  const MATERIALS = [
    { class: 'material-chrome', name: 'material-chrome', use: 'Navigation bar' },
    { class: 'material-panel', name: 'material-panel', use: 'Side panel' },
    { class: 'material-overlay', name: 'material-overlay', use: 'Menu, popover, tip' },
  ] as const;
</script>

<div class="stack">
  <h3>Materials: chrome, panel and overlay glass over a busy backdrop; solid under reduced transparency or increased contrast</h3>
  <div class="backdrop">
    <p class="noise" aria-hidden="true">Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima Anima</p>
    {#each MATERIALS as material (material.name)}
      <div class="sample {material.class}">
        <strong>{material.name}</strong>
        <span>{material.use}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .stack {
    display: grid;
    gap: var(--space-3);
  }

  h3 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  .backdrop {
    position: relative;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-6);
    align-items: center;
    padding: var(--space-8) var(--space-6);
    overflow: hidden;
    border-radius: var(--radius-lg);
    background: repeating-linear-gradient(135deg, var(--accent) 0 var(--space-4), var(--status-down) var(--space-4) var(--space-8), var(--status-up) var(--space-8) var(--space-12), var(--surface-raised) var(--space-12) var(--space-16));
  }

  .noise {
    position: absolute;
    inset: 0;
    margin: 0;
    color: var(--text);
    font-size: var(--text-2xl);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
    word-break: break-all;
  }

  .sample {
    position: relative;
    box-sizing: border-box;
    display: grid;
    gap: var(--space-1);
    padding: var(--space-4);
    border-radius: var(--radius-md);
    color: var(--text);
  }

  span {
    font-size: var(--text-sm);
  }
</style>
