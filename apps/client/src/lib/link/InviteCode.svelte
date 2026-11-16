<script lang="ts">
  import CopyText from '$lib/forms/copy/CopyText.svelte';
  import QrCode from '$lib/qr/QrCode.svelte';
  import type { LinkInvite } from './api';
  import { countdownText, secondsLeft } from './devices';

  interface Props {
    /** The invite. Its text is drawn and copied only: never logged, put in an attribute or announced (LOG-007). */
    invite: LinkInvite;
    /** The device it is for. */
    name: string;
    now: Date;
  }

  const { invite, name, now }: Props = $props();
  const left = $derived(secondsLeft(invite.expiresAt, now));
</script>

<div class="invite">
  <div class="pair">
    <QrCode text={invite.invite} label="Invite for {name}" />
    <div class="side">
      <p>On {name}, open Anima.ing, choose "Join a hub" and scan or paste this invite. It works once.</p>
      <!-- Not a live region: a countdown read out every second would drown the page. -->
      <p class="left">
        {#if left > 0}Expires in <time datetime={invite.expiresAt}>{countdownText(left)}</time>{:else}This invite has expired. Make another.{/if}
      </p>
    </div>
  </div>
  {#if left > 0}<CopyText label="Invite" text={invite.invite} />{/if}
</div>

<style>
  .invite {
    display: grid;
    gap: var(--space-3);
  }

  .pair {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--space-4);
    align-items: center;
  }

  .side {
    display: grid;
    gap: var(--space-2);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .left {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }
</style>
