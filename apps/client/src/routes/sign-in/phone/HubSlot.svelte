<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    host: string;
    /** The hub does not answer: the slot says so and offers another try (MOB-018, MOB-037). */
    unreachable: boolean;
    /** Open the hub address field. */
    onchange: () => void;
    onretry: () => void;
  }

  const { host, unreachable, onchange, onretry }: Props = $props();
</script>

<!-- The hub slot of the phone layout (mockup 3): one row of a fixed height under the button. It names the hub, or says
     that the hub does not answer; because its height never changes, the notice moves no control (MOB-018). -->
<div class={['slot', unreachable && 'bad']} role="status">
  <Icon name={unreachable ? 'carbon:unlink' : 'carbon:network-3'} size="lg" />
  <p><b>{unreachable ? `Can't reach ${host}` : host}</b>{unreachable ? "Check this device's network." : 'Hub address'}</p>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
  <button type="button" tabindex="0" onclick={unreachable ? onretry : onchange} {@attach press()}>{unreachable ? 'Retry' : 'Change'}</button>
</div>

<style>
  /* The sizes are those of mockup 3: 68 px high, 12 px padding and gap, 14 px text. */
  .slot {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    block-size: calc(var(--space-16) + var(--space-1));
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--text-md);
    line-height: 1.35;
  }

  .bad {
    background: var(--status-down-tint);
  }

  .bad :global(.icon) {
    color: var(--status-down-text);
  }

  p {
    flex: 1;
    min-inline-size: 0;
    margin: 0;
  }

  b {
    display: block;
    overflow: hidden;
    color: var(--text);
    font-weight: var(--weight-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button {
    flex: none;
    min-block-size: var(--target-min-touch);
    padding: 0 calc(var(--space-2) + var(--border-focus));
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: var(--weight-medium);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
