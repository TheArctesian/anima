<script lang="ts">
  import { accessApi } from '$lib/access/api';
  import { signInNeeded } from '$lib/access/guard';
  import { leaveHub } from '$lib/access/leave/leave';
  import { goToSignIn } from '$lib/access/navigation';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  // The Account rows of the phone layout (mockup 7): the user name, whether two-factor is on, and Sign out.
  const hub = useHub();
  const api = accessApi(hub);
  let me = $state<{ readonly username: string; readonly totpEnabled: boolean } | null>(null);
  let problem = $state('');
  let busy = $state(false);

  $effect(() => {
    void api.me().then((result) => {
      if (result.ok) me = result.value;
      else if (!signInNeeded(result.error, goToSignIn)) problem = "Can't read who is signed in.";
    });
  });

  async function leave(): Promise<void> {
    if (busy) return;
    busy = true;
    // In a browser behind the gateway, this also signs out of the gateway (HMD-059).
    const kept = await leaveHub(hub, api);
    busy = false;
    if (kept !== null) problem = kept;
  }
</script>

<div class="phone-kv"><span>User name</span><span class="v"><span>{me?.username ?? ''}</span></span></div>
<div class="phone-kv">
  <span>Two-factor</span>
  <span class="v">{#if me !== null}<span class="dot" class:up={me.totpEnabled} aria-hidden="true"></span><span>{me.totpEnabled ? 'On' : 'Off'}</span>{/if}</span>
</div>
<button type="button" class="phone-kv" tabindex="0" aria-busy={busy} onclick={() => void leave()} {@attach press()}>
  <span>Sign out</span><span class="v"><Icon name="carbon:chevron-right" size="sm" /></span>
</button>
{#if problem !== ''}<p class="phone-kv-problem" role="alert">{problem}</p>{/if}

