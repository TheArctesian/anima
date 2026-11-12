<script lang="ts">
  import { accessApi } from '$lib/access/api';
  import { goToSignIn } from '$lib/access/navigation';
  import { signOut } from '$lib/access/sign-out';
  import { clearAccessToken } from '$lib/desktop/token';
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
  import { CHECKING, connectionSentence } from './hub-check';

  // Desktop only; the page decides (CLI-002, D-5). The address is the one the probe contacts and sign-in starts from; it is not a secret.
  const hub = useHub();
  const api = accessApi(hub);
  let status = $state('');
  let busy = $state(false);

  async function test(): Promise<void> {
    if (busy) return;
    busy = true;
    status = CHECKING;
    status = connectionSentence(await api.me());
    busy = false;
  }

  /** Changing the hub signs out, then the sign-in form, which has the address field, takes over (CLI-002). */
  async function change(): Promise<void> {
    if (busy) return;
    busy = true;
    status = '';
    // `signOut` returns fixed sentences and does not throw; its own messages are shown whole, as SignOutRow shows them.
    const kept = await signOut({ api, address: hub.address, desktop: true, clearToken: clearAccessToken });
    if (kept === null) goToSignIn();
    else status = kept;
    busy = false;
  }
</script>

<div class="row">
  <!-- Two fixed lines: the label, then the address on its own line, cut with an ellipsis so a long host never wraps (MOT-007). -->
  <p class="address">Hub address<strong>{hub.address.origin}</strong></p>
  <div class="buttons">
    <!-- aria-disabled, not disabled: a busy button keeps focus and its width (MOT-007). -->
    <button type="button" tabindex="0" aria-disabled={busy} aria-busy={busy} onclick={test} {@attach press()}>Test connection</button>
    <button type="button" tabindex="0" aria-disabled={busy} aria-busy={busy} onclick={change} {@attach press()}>Change hub</button>
  </div>
  <!-- Always mounted with its line reserved, so a result moves nothing (MOT-007). -->
  <p role="status" class="status">{status}</p>
</div>

<style>
  .row {
    display: flex;
    flex-direction: column;
    align-items: start;
    gap: var(--space-2);
    margin-block-start: var(--space-6);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  .address {
    min-block-size: calc(var(--text-md) * var(--leading-tight) * 2);
    max-inline-size: 100%;
    overflow: hidden;
    white-space: nowrap;
  }

  .address strong {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  /* Four lines, as in SignOutRow: the longest sign-out message takes four at 390 px (MOT-007). */
  .status {
    min-block-size: calc(4 * var(--text-md) * var(--leading-tight));
  }

  button {
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition:
      background-color var(--duration-fast) var(--ease-enter),
      opacity var(--duration-fast) var(--ease-enter);
  }

  button:hover:not([aria-disabled='true']) {
    background: var(--surface-overlay);
  }

  button[aria-disabled='true'] {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
