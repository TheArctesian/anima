<script lang="ts">
  import { drag } from '$lib/motion/drag/drag';
  import { draggedWidth, holdWidth, restsCollapsed, type BarWidths } from './resize';

  interface Props {
    collapsed: boolean;
    /** Called when a drag or a key settles the bar; the caller collapses or expands it. */
    onsettle: (collapsed: boolean) => void;
  }

  const { collapsed, onsettle }: Props = $props();
  const root = document.documentElement;
  let widths: BarWidths = { icons: 0, full: 0 };

  /** The bar's width in each state, read from the layout without a paint between. */
  function measure(edge: HTMLElement): BarWidths {
    const bar = edge.parentElement;
    if (bar === null) return widths;
    const was = root.dataset['nav'];
    const widthAs = (nav: string | undefined): number => {
      if (nav === undefined) delete root.dataset['nav'];
      else root.dataset['nav'] = nav;
      return bar.getBoundingClientRect().width;
    };
    const measured = { icons: widthAs('icons'), full: widthAs(undefined) };
    widthAs(was);
    return measured;
  }

  function key(event: KeyboardEvent): void {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();
    onsettle(event.key === 'ArrowLeft');
  }
</script>

<!-- 5.7.4: the bar's edge follows the pointer 1:1; on release it rests at the width the throw carries it to. -->
<!-- A focusable separator is the ARIA window-splitter pattern: it takes the arrow keys. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="edge"
  role="separator"
  aria-orientation="vertical"
  aria-label="Resize sidebar"
  aria-controls="main-nav"
  aria-valuemin={0}
  aria-valuemax={1}
  aria-valuenow={collapsed ? 0 : 1}
  aria-valuetext={collapsed ? 'Icons only' : 'Icons and names'}
  tabindex="0"
  onkeydown={key}
  {@attach (edge) =>
    drag({
      axis: 'x',
      origin: () => edge.getBoundingClientRect().left + edge.getBoundingClientRect().width / 2,
      onGrab: () => {
        widths = measure(edge);
      },
      onMove: ({ position }) => {
        holdWidth(root, draggedWidth(position, widths));
      },
      onRelease: ({ position, velocity }) => {
        onsettle(restsCollapsed(position, velocity, widths));
      },
    })(edge)}
></div>

<style>
  /* A strip over the bar's border: wide enough to grab, drawn only as the border it sits on. */
  .edge {
    position: absolute;
    inset-block: 0;
    inset-inline-end: calc(var(--space-1) * -1);
    inline-size: var(--space-2);
    cursor: col-resize;
    touch-action: none;
  }

  .edge:hover,
  .edge:focus-visible {
    background: var(--accent-tint);
  }

  .edge:focus-visible {
    outline: var(--border-focus) solid var(--accent);
  }
</style>
