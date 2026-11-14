<script lang="ts">
  import SmallCaps from '$lib/page/SmallCaps.svelte';
</script>

<!-- One row per text style of the website's language (UR-7, D1, D2, D5): its name, then a sample in it. -->
<dl>
  <div><dt><SmallCaps>Display · 200</SmallCaps></dt><dd class="display">Every machine you own</dd></div>
  <div><dt><SmallCaps>XL · 400</SmallCaps></dt><dd class="xl">One private place that you run</dd></div>
  <div><dt><SmallCaps>Body · 400</SmallCaps></dt><dd>Status of machines, cloud devices and servers, in one view.</dd></div>
  <div><dt><SmallCaps>Emphasis · 500</SmallCaps></dt><dd class="medium">atlas is up, 99.9% over 24 hours</dd></div>
  <div><dt><SmallCaps>Label · 500 caps</SmallCaps></dt><dd><SmallCaps>Download for macOS</SmallCaps></dd></div>
  <div><dt><SmallCaps>Mono · Plex</SmallCaps></dt><dd class="mono">ssh ops@atlas.lan -p 22 · 100.64.0.12</dd></div>
  <div><dt><SmallCaps>Quiet</SmallCaps></dt><dd class="quiet">Decoration only, never the only carrier of information</dd></div>
</dl>

<style>
  dl {
    margin: 0;
  }

  div {
    display: grid;
    grid-template-columns: 9rem minmax(0, 1fr);
    gap: var(--space-6);
    align-items: baseline;
    padding-block: var(--space-3);
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  dt {
    align-self: center;
  }

  dd {
    margin: 0;
    min-inline-size: 0;
  }

  .display {
    font-size: var(--text-display);
    font-weight: var(--weight-light);
    letter-spacing: var(--tracking-tight);
    line-height: var(--leading-tight);
  }

  .xl {
    font-size: var(--text-xl);
  }

  .medium {
    font-weight: var(--weight-medium);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .quiet {
    color: var(--text-quiet);
  }

  @media (max-width: 767.98px) {
    div {
      grid-template-columns: minmax(0, 1fr);
      gap: var(--space-1);
    }
  }
</style>
