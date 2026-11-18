<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { elapsedText } from './elapsed';

  interface Props {
    name: string;
    /** The session is open; the time counts from the moment it opened. */
    ready: boolean;
    /** Shows the pages; the session stays open in Sessions (D-11). */
    onback: () => void;
    onhidekeyboard: () => void;
    /** The session menu (the keys and Paste). */
    menu: Snippet;
  }

  const { name, ready, onback, onhidekeyboard, menu }: Props = $props();
  let openedAt: number | undefined = $state();
  let now = $state(Date.now());

  $effect(() => {
    if (!ready) {
      openedAt = undefined;
      return;
    }
    openedAt ??= Date.now();
    now = Date.now();
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => {
      clearInterval(timer);
    };
  });
</script>

<!-- The session's bar in the phone layout (mockup 8): back, the device and the time, the keyboard and the menu. -->
<div class="phone-session-bar">
  <button type="button" class="iconbtn" aria-label="Back to devices" onclick={onback} {@attach press()}><Icon name="carbon:chevron-left" /></button>
  <div class="title">
    <b>{name}</b>
    <small><span class="dot" class:up={ready} aria-hidden="true"></span>SSH{#if openedAt !== undefined}&nbsp;{elapsedText(now - openedAt)}{/if}</small>
  </div>
  <!-- It does not take the focus from the terminal: the terminal lets it go, and the keyboard closes. -->
  <button type="button" class="iconbtn" aria-label="Hide keyboard" onmousedown={(event) => event.preventDefault()} onclick={onhidekeyboard} {@attach press()}><Icon name="carbon:chevron-down" /></button>
  {@render menu()}
</div>
