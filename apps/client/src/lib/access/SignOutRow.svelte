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

  $effect(() => {
    void api.me().then((result) => {
      if (result.ok) username = result.value.username;
      else if (!signInNeeded(result.error, goToSignIn)) problem = 'Anima could not read who is signed in.';
    });
  });

  async function leave(): Promise<void> {
    const kept = await signOut({ api, address: hub.address, clearToken: clearAccessToken });
    if (kept === null) goToSignIn();
    else problem = kept;
  }
</script>

<section aria-label="Account">
  {#if username !== null}
    <p>Signed in as <strong>{username}</strong></p>
  {/if}
  <button type="button" onclick={leave} {@attach press()}>Sign out</button>
  {#if problem !== null}
    <p role="alert">{problem}</p>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
    margin-block-start: var(--space-6);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
  }

  button {
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-raised);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  [role='alert'] {
    flex-basis: 100%;
    color: var(--status-down);
  }
</style>
