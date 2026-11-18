<script lang="ts">
  import { accessApi } from '$lib/access/api';
  import { useHub } from '$lib/hub/context';

  // The Hub row of the phone layout (mockup 7): a dot for whether the hub answers, and the hub. The API has no hub name,
  // so the row shows the address. The Link and This device rows wait for the link on a phone (mobile plan M3-1).
  const hub = useHub();
  const api = accessApi(hub);
  let reached = $state<boolean | null>(null);
  const host = $derived(new URL(hub.address.origin).host);

  $effect(() => {
    void api.me().then((result) => {
      reached = result.ok || result.error.code === 'UNAUTHENTICATED';
    });
  });
</script>

<div class="phone-kv">
  <span>Hub</span>
  <span class="v"
    ><span class="dot" class:up={reached === true} class:down={reached === false} aria-hidden="true"></span><span
      >{host}</span
    ></span
  >
</div>
