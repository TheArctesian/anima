<script lang="ts">
  import { untrack } from 'svelte';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import { animate } from '$lib/motion/animate';
  import { drag } from '$lib/motion/drag/drag';
  import { tweenMotion } from '$lib/motion/options';
  import { motionPreference } from '$lib/motion/preference';
  import { press } from '$lib/motion/roles/press';
  import { LogFeed } from './feed.svelte';
  import LogFilters from './LogFilters.svelte';
  import LogList from './LogList.svelte';
  import { PanelPlacement } from '../resize/placement.svelte';
  import '../resize/geometry.css';
  import { snapsOf } from '../resize/resize';

  interface Props {
    /** The devices the page already has, for the device filter. */
    devices: readonly { id: string; name: string }[];
    /** The time of a load and of each live event; tests pass their own. */
    now?: () => Date;
  }

  const { devices, now = () => new Date() }: Props = $props();
  const id = $props.id();
  const preference = motionPreference();
  const feed = new LogFeed(useHub(), () => now());
  let pageHeight = $state(innerHeight);
  let header = $state<HTMLElement>();
  let headerHeight = $state(0);
  let panel = $state<HTMLElement>();
  let edge = $state<HTMLElement>();
  let content = $state<HTMLElement>();
  const snaps = $derived(snapsOf(headerHeight, pageHeight));
  const placement = new PanelPlacement({ snaps: () => snaps, panel: () => panel, preference });
  let fadedIn: boolean | undefined;

  /** Starting reads the filter; only the panel coming and going restarts it, a filter change goes through `setFilter`. */
  $effect(() => {
    untrack(() => {
      feed.start();
    });
    return () => {
      feed.stop();
    };
  });

  /** The header has a fixed block size from tokens, so it is measured when the page changes size, not watched. */
  $effect(() => {
    void pageHeight;
    headerHeight = header?.offsetHeight ?? 0;
  });

  /** The panel sits at its stop on first paint and when the page changes size. */
  $effect(() => {
    void snaps;
    untrack(() => {
      placement.settle();
    });
  });

  /** The content fades in and out with the panel; reduced motion keeps the fade (5.7.3, Disclosure). */
  $effect(() => {
    const open = placement.open;
    const was = fadedIn;
    fadedIn = open;
    if (content === undefined || was === undefined || was === open) return;
    animate(content, { opacity: open ? 1 : 0 }, tweenMotion('base', 'enter', preference.reduced));
  });

  const resize = drag({
    axis: 'y',
    origin: () => edge?.getBoundingClientRect().top ?? 0,
    onGrab: () => {
      placement.grab();
    },
    onMove: ({ delta }) => {
      placement.move(delta);
    },
    onRelease: ({ velocity }) => {
      placement.release(-velocity);
    },
  });
</script>

<svelte:window bind:innerHeight={pageHeight} />

<section class="panel" aria-labelledby="{id}-title" bind:this={panel} style:block-size="{snaps[2]}px">
    <!-- A focusable separator with a value is the WAI-ARIA window splitter, an interactive widget that Svelte's check does not know. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="edge"
      role="separator"
      tabindex="0"
      aria-orientation="horizontal"
      aria-label="Resize logs panel"
      aria-valuemin={snaps[0]}
      aria-valuemax={snaps[2]}
      aria-valuenow={Math.round(placement.height)}
      bind:this={edge}
      onkeydown={(event) => {
        if (placement.press(event.key)) event.preventDefault();
      }}
      {@attach resize}
    ></div>
    <header bind:this={header}>
      <h2 id="{id}-title">Logs</h2>
      <button
        type="button"
        tabindex="0"
        aria-expanded={placement.open}
        onclick={() => {
          placement.toggle();
        }}
        {@attach press()}
      >
        <Icon name={placement.open ? 'nf-md-chevron_down' : 'nf-md-chevron_right'} size="sm" />
        <span>{placement.open ? 'Collapse' : 'Expand'}</span>
      </button>
    </header>
    <div class="content" bind:this={content} inert={!placement.open}>
      <LogFilters filter={feed.filter} {devices} onChange={(next) => { feed.setFilter(next); }} />
      <!-- A scrollable region must take focus so the keyboard can scroll it. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="scroll" role="region" aria-label="Log list" tabindex="0"><LogList {feed} {now} /></div>
    </div>
</section>

<style>
  /*
   * Pinned to the viewport at the bottom of the content column, beside the navigation bar: it takes no room in the page and
   * a translate never adds scroll height. Until the script has placed it, it is below the screen, never in the flow.
   */
  .panel {
    position: fixed;
    inset-block-end: 0;
    inset-inline: var(--nav-inline-size) 0;
    z-index: var(--layer-sticky);
    translate: 0 100%;
    display: flex;
    flex-direction: column;
    border-block-start: var(--border-thin) solid var(--border-strong);
    background: var(--material-panel-bg);
    backdrop-filter: blur(var(--material-panel-blur)) saturate(var(--material-panel-saturate));
    box-shadow: var(--elevation-2);
  }

  .edge {
    position: absolute;
    inset-block-start: calc(var(--space-1) * -1);
    inset-inline: 0;
    block-size: var(--space-2);
    cursor: row-resize;
    touch-action: none;
  }

  .edge:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  header {
    box-sizing: border-box;
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    block-size: var(--logs-header-size);
    padding: 0 var(--space-3);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
  }

  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-overlay);
  }

  button:active {
    transition-duration: var(--duration-press);
    background: var(--surface-hover);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .content {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-2);
    min-block-size: 0;
    min-inline-size: 0;
    padding: 0 var(--space-3) var(--space-3);
  }

  .scroll {
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .scroll:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
