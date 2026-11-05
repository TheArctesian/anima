<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { devicesApi, type CardDevice } from '$lib/devices/api';
  import { parseListQuery, writeListQuery, type ListQuery } from '$lib/devices/query';
  import DeviceScreen from '$lib/devices/screen/DeviceScreen.svelte';
  import { addDevice as createWithCredential, type AddedDevice } from '$lib/forms/add-device';
  import AddDeviceModal from '$lib/forms/AddDeviceModal.svelte';
  import type { DeviceFormValues, SiteOption } from '$lib/forms/device/values';
  import { useHub } from '$lib/hub/context';
  import type { HubError } from '$lib/hub/error';
  import { sessionsApi, type ConnectionTestInput } from '$lib/sessions/api';
  import { useSessions } from '$lib/sessions/sessions.svelte';

  const sessions = useSessions();
  const hub = useHub();
  const devices = devicesApi(hub);
  const connections = sessionsApi(hub);

  let sites = $state<readonly SiteOption[]>([]);
  let adding = $state(false);
  let addError = $state<HubError | null>(null);

  const query = $derived(parseListQuery(page.url.searchParams));

  /** The query goes to the URL, so Back and Forward restore the list (D-4). */
  function changeQuery(next: ListQuery): void {
    const search = writeListQuery(next).toString();
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the query string is added
    void goto(`${resolve('/devices')}${search === '' ? '' : `?${search}`}`, {
      replaceState: true,
      keepFocus: true,
      noScroll: true,
    });
  }

  /** Add device (DEV-010, WP-13 H-1): the sites are read first, because the form needs a site to place the device in. */
  async function addDevice(): Promise<void> {
    const result = await devices.sites();
    addError = result.ok ? null : result.error;
    if (!result.ok) return;
    sites = result.value;
    adding = true;
  }

  function create(values: DeviceFormValues, siteId: string) {
    return createWithCredential(values, siteId, {
      createDevice: (input) => devices.create(input),
      setSshCredential: (deviceId, credential) => devices.setSshCredential(deviceId, credential),
    });
  }

  function test(input: ConnectionTestInput) {
    return connections.testConnection(input);
  }

  function created(added: AddedDevice): void {
    adding = false;
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); the ID is the hub's and is encoded
    void goto(`${resolve('/devices')}/${encodeURIComponent(added.id)}`);
  }

  /** Opens an SSH tab that grows from the button (WP-14 H-1, MOT-005). */
  function connect(device: CardDevice, button: HTMLElement): void {
    sessions.openSession({
      deviceId: device.id,
      deviceName: device.name,
      protocol: 'SSH',
      origin: button.getBoundingClientRect(),
    });
  }
</script>

<section>
  <h1>Devices</h1>
  {#if addError !== null}
    <p class="notice" role="alert">Could not open Add device: {addError.message}</p>
  {/if}
  <DeviceScreen {query} onQueryChange={changeQuery} onAddDevice={() => void addDevice()} onConnect={connect} />
</section>

<AddDeviceModal
  open={adding}
  {sites}
  onCreate={create}
  onTest={test}
  onClose={() => {
    adding = false;
  }}
  onCreated={created}
/>

<style>
  section {
    min-inline-size: 0;
  }

  .notice {
    margin: 0 0 var(--space-4);
    color: var(--text);
  }

  /* A fixed block size from tokens, so the font swap cannot move the page (WP-11 H-7). */
  h1 {
    box-sizing: border-box;
    block-size: calc(var(--text-2xl) * var(--leading-tight));
    margin: 0 0 var(--space-4);
    overflow: hidden;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    white-space: nowrap;
  }
</style>
