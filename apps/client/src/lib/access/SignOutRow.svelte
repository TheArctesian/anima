<script lang="ts">
  import { useHub } from '$lib/hub/context';
  import { clearAccessToken } from '$lib/desktop/token';
  import { press } from '$lib/motion/roles/press';
  import { accessApi } from './api';
  import { signInNeeded } from './guard';
  import { goToSignIn } from './navigation';
  import { signOut } from './sign-out';

  const hub = useHub();
  const api = accessApi(hub);
  let username = $state<string | null>(null);
  let problem = $state<string | null>(null);
  let busy = $state(false);

  $effect(() => {
    void api.me().then((result) => {
      if (result.ok) username = result.value.username;
      else if (!signInNeeded(result.error, goToSignIn)) problem = 'Anima could not read who is signed in.';
    });
  });

  async function leave(): Promise<void> {
    busy = true;
    let kept: string | null;
    try {
      kept = await signOut({ api, address: hub.address, clearToken: clearAccessToken });
    } finally {
      busy = false;
    }
    if (kept === null) goToSignIn();
    else problem = kept;
  }
</script>

<div class="row">
  <!-- Always rendered with two lines reserved (the label, then the name on its own line), so the button does not move when the name arrives, however narrow the column is (MOT-007). -->
  <p class="who" class:pending={username === null}>Signed in as <strong>{username ?? ''}</strong></p>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable, as for the nav links (NAV-005, WP-11 H-6). -->
  <button type="button" tabindex="0" disabled={busy} onclick={leave} {@attach press()}>Sign out</button>
  {#if problem !== null}
    <p role="alert">{problem}</p>
  {/if}
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
  }

  /* Two lines are reserved; the name never wraps, it is cut with an ellipsis, so its arrival cannot change the height (MOT-007). */
  .who {
    min-block-size: calc(var(--text-md) * var(--leading-tight) * 2);
    max-inline-size: 100%;
    overflow: hidden;
    line-height: var(--leading-tight);
    white-space: nowrap;
  }

  .who.pending {
    visibility: hidden;
  }

  .who strong {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  button {
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-raised);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
