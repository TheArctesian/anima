<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { untrack } from 'svelte';
  import { page } from '$app/state';
  import { desktopWebView } from '$lib/desktop/webView/webView';
  import { readThisMachine, type ThisMachine } from '$lib/desktop/thisMachine';
  import { checksApi } from '$lib/checks/api';
  import type { CardDevice } from '$lib/devices/api';
  import { devicesApi } from '$lib/devices/client';
  import { parseListQuery, writeListQuery, type ListQuery } from '$lib/devices/query';
  import DeviceScreen from '$lib/devices/screen/DeviceScreen.svelte';
  import { addDevice as createWithCredential, type AddedDevice } from '$lib/forms/add-device';
  import AddDeviceModal from '$lib/forms/AddDeviceModal.svelte';
  import type { SitePlacement } from '$lib/forms/device/site';
  import type { DeviceFormValues, SiteOption } from '$lib/forms/device/values';
  import { machineToOffer, offeredMachine, thisMachinePrompt, type MachineOffer } from '$lib/forms/this-machine';
  import { useHub } from '$lib/hub/context';
  import type { HubError, HubResult } from '$lib/hub/error';
  import { sessionsApi } from '$lib/sessions/api';
  import { wakeApi } from '$lib/sessions/wake/api';
  import { useSessions } from '$lib/sessions/sessions.svelte';
  import { openWebInterface } from '$lib/sessions/web/open';
  import type { SessionProtocol } from '$lib/sessions/tabs';

  const sessions = useSessions();
  const hub = useHub();
  const devices = devicesApi(hub);
  const checks = checksApi(hub);
  const connections = sessionsApi(hub);
  const wake = wakeApi(hub);

  let sites = $state<readonly SiteOption[]>([]);
  let adding = $state(false);
  let addError = $state<HubError | null>(null);
  /** This computer when it is known and not yet a device, so Add device offers it first (R-060). */
  let machine = $state<ThisMachine | null>(null);
  /** The app opened Add device itself, to ask about `machine`. */
  let asking = $state(false);
  let loadingSites = false;

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

  /** Shows Add device over the list with the sites it needs, offering `offered` first when it is not null. */
  function show(found: HubResult<readonly SiteOption[]>, offered: ThisMachine | null, ask: boolean): void {
    addError = found.ok ? null : found.error;
    if (!found.ok) return;
    sites = found.value;
    machine = offered;
    asking = ask;
    adding = true;
  }

  /** Once this computer is found to be a device, the prompt stops looking for the rest of the run. */
  function remember(offer: MachineOffer): ThisMachine | null {
    if (offer.kind === 'device') thisMachinePrompt.isADevice();
    return offeredMachine(offer);
  }

  /**
   * Add device (DEV-010, WP-13 H-1): the sites are read first, because the form needs a site to place the device
   * in. This computer is read at the same time, so the form can offer it (R-060).
   */
  async function addDevice(): Promise<void> {
    if (loadingSites || adding) return;
    loadingSites = true;
    const [found, offer] = await Promise.all([devices.sites(), machineToOffer(readThisMachine, devices.hosts)]);
    loadingSites = false;
    show(found, remember(offer), false);
  }

  /**
   * After sign-in, Add device opens by itself, once per app start, when this computer is known and not yet a
   * device, unless the operator said not to ask again (R-060). A failure to read the sites stays quiet here.
   */
  async function askAboutThisMachine(): Promise<void> {
    if (!thisMachinePrompt.due || loadingSites || adding) return;
    loadingSites = true;
    const offered = remember(await machineToOffer(readThisMachine, devices.hosts));
    const found = offered === null ? null : await devices.sites();
    loadingSites = false;
    // The operator may have turned the prompt off, or opened Add device, while the reads were pending.
    if (offered === null || found === null || !found.ok || !thisMachinePrompt.due || adding) return;
    thisMachinePrompt.asked();
    show(found, offered, true);
  }

  // Once, when the page mounts; nothing the prompt reads should run it again.
  $effect(() => {
    untrack(() => void askAboutThisMachine());
  });

  function create(values: DeviceFormValues, site: SitePlacement) {
    return createWithCredential(values, site, {
      sites: () => devices.sites(),
      createSite: (name, subnet) => devices.createSite(name, subnet),
      createDevice: (input) => devices.create(input),
      setSshCredential: (deviceId, credential) => devices.setSshCredential(deviceId, credential),
      createCheck: (input) => checks.create(input),
    });
  }

  function created(added: AddedDevice): void {
    adding = false;
    // The hub's message goes in page state, not the URL, so a reload does not replay it and no secret is involved (LOG-007).
    const state = added.credentialError === null ? {} : { credentialError: added.credentialError.message };
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); the ID is the hub's and is encoded
    void goto(`${resolve('/devices')}/${encodeURIComponent(added.id)}`, { state });
  }

  /** Opens a tab for the protocol of the button that grows from it (WP-14 H-1, RA-030, MOT-005). */
  function connect(device: CardDevice, protocol: SessionProtocol, button: HTMLElement): void {
    sessions.openSession({
      deviceId: device.id,
      deviceName: device.name,
      protocol,
      origin: button.getBoundingClientRect(),
    });
  }

  /** Opens the interface as its setting says: a web tab that grows from the button, or the system browser (WEB-003, WEB-011). */
  function openWeb(device: CardDevice, webInterface: CardDevice['webInterfaces'][number], button: HTMLElement): void {
    void openWebInterface(sessions, desktopWebView, { device, webInterface, origin: button.getBoundingClientRect() });
  }
</script>

<section>
  <h1>Devices</h1>
  {#if addError !== null}
    <p class="notice" role="alert">Could not open Add device: {addError.message}</p>
  {/if}
  <DeviceScreen {query} onQueryChange={changeQuery} onAddDevice={() => void addDevice()} onConnect={connect} onOpenWeb={openWeb} {wake} />
</section>

<AddDeviceModal
  open={adding}
  {sites}
  {machine}
  {asking}
  onDontAsk={() => {
    thisMachinePrompt.turnOff();
    adding = false;
  }}
  onCreate={create}
  onTest={connections.testConnection}
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
