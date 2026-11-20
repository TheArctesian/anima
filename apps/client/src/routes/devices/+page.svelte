<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { untrack } from 'svelte';
  import { page } from '$app/state';
  import { readThisMachine } from '$lib/desktop/this-machine';
  import { readLink } from '$lib/desktop/link/shell';
  import type { CardDevice } from '$lib/devices/api';
  import { devicesApi } from '$lib/devices/client';
  import { parseListQuery, writeListQuery, type ListQuery } from '$lib/devices/query';
  import DeviceScreen from '$lib/devices/screen/DeviceScreen.svelte';
  import { readCloudDevice } from '$lib/desktop/cloud/device';
  import { hubIsHosted } from '$lib/hosted/mark';
  import { enrolAtAddress, enrolThisComputer, type DeviceEnrolment } from '$lib/hosted/this-computer';
  import { enrolLinkedDevice } from '$lib/devices/enrollment/enrol-linked-device';
  import { clearPendingEnrolment, pendingEnrolment } from '$lib/devices/enrollment/pending-enrolment';
  import { VPN_SITE } from '$lib/devices/form/site';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import InviteMaker from '$lib/link/InviteMaker.svelte';
  import { createClock } from '$lib/nav/clock.svelte';
  import { isAndroid, isIPad } from '$lib/platform/mobile';
  import { linkApi } from '$lib/link/api';
  import { useHub } from '$lib/hub/context';
  import { screenSharingCalls } from '$lib/devices/screen-sharing/calls';
  import { wakeApi } from '$lib/sessions/wake/api';
  import { useSessions } from '$lib/sessions/sessions.svelte';
  import { openWebInterface } from '$lib/web-interfaces/open/open';
  import { webViewShell } from '$lib/web-interfaces/shell/client-shell';
  import { capabilities } from '$lib/platform/platform';
  import type { SessionProtocol } from '$lib/sessions/tabs';

  const sessions = useSessions();
  const hub = useHub();
  const devices = devicesApi(hub);
  const wake = wakeApi(hub);
  const screenSharing = screenSharingCalls(hub);

  const link = linkApi(hub);
  const clock = createClock(1000);
  const platform = capabilities();
  const deviceName = isIPad(navigator) ? 'iPad' : isAndroid(navigator) ? 'Android device' : 'iPhone';
  const selfLabel = platform.thisMachine ? 'Enroll this computer' : `Enroll this ${deviceName}`;
  const canEnrolHere = platform.thisMachine || platform.deviceLink;
  const inventory = { hosts: devices.hosts, sites: devices.sites, createSite: devices.createSite, createDevice: devices.create };
  const hostedComputer = { ...inventory, device: readCloudDevice, linkDevices: link.devices };
  const currentLink = { ...inventory, link: readLink, linkDevices: link.devices };
  const mobileDevice = { name: deviceName, kind: deviceName === 'iPad' ? 'tablet' : 'phone' };
  let panel = $state<'choose' | 'invite' | null>(null);
  let busy = $state(false);
  let result = $state<DeviceEnrolment | null>(null);
  let listVersion = $state(0);

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

  async function enrollHere(): Promise<DeviceEnrolment> {
    if (!platform.thisMachine) {
      return enrolLinkedDevice(hub.address.origin, currentLink, mobileDevice);
    }
    if (hubIsHosted(hub.address.origin)) return enrolThisComputer(hostedComputer);
    const machine = await readThisMachine(hub);
    if (machine === null) return enrolLinkedDevice(hub.address.origin, currentLink, { name: 'Computer', kind: 'workstation' });
    return enrolAtAddress(inventory, VPN_SITE, {
      name: machine.name.trim() || 'Computer', host: machine.vpnAddress, kind: 'workstation', os: machine.os, tags: [],
    });
  }

  async function enroll(): Promise<void> {
    if (busy) return;
    busy = true;
    result = null;
    try {
      result = await enrollHere();
      if (result === 'added') listVersion += 1;
    } catch {
      result = 'failed';
    } finally {
      busy = false;
    }
  }

  function close(): void {
    if (!busy) panel = null;
  }

  async function enrollAfterSignIn(): Promise<void> {
    if (!platform.deviceLink) return;
    const invitedKey = pendingEnrolment(sessionStorage, hub.address.origin);
    if (invitedKey !== undefined) {
      const computer = platform.thisMachine ? await readCloudDevice() : null;
      const device = platform.thisMachine
        ? { name: computer?.computer || 'Computer', kind: 'workstation', os: computer?.os }
        : mobileDevice;
      const outcome = await enrolLinkedDevice(hub.address.origin, currentLink, { ...device, invitedKey });
      if (outcome === 'added' || outcome === 'already-added') clearPendingEnrolment(sessionStorage);
      if (outcome === 'added') listVersion += 1;
    } else if (platform.thisMachine && hubIsHosted(hub.address.origin) && await enrolThisComputer(hostedComputer) === 'added') {
      listVersion += 1;
    }
  }

  $effect(() => {
    untrack(() => void enrollAfterSignIn().catch(() => undefined));
  });

  /** Opens a tab for the protocol of the button that grows from it (WP-14 H-1, RA-030, MOT-005). */
  function connect(device: CardDevice, protocol: SessionProtocol, button: HTMLElement): void {
    sessions.openSession({
      deviceId: device.id,
      deviceName: device.name,
      os: device.os,
      ...(device.distribution == null ? {} : { distribution: device.distribution }),
      protocol,
      origin: button.getBoundingClientRect(),
    });
  }

  /** Opens the interface as its setting says: a web tab that grows from the button, or the system browser (WEB-003, WEB-011). */
  function openWeb(device: CardDevice, webInterface: CardDevice['webInterfaces'][number], button: HTMLElement): void {
    void openWebInterface(sessions, webViewShell(), { device, webInterface, origin: button.getBoundingClientRect() });
  }
</script>

<section>
  {#key listVersion}
    <DeviceScreen {query} onQueryChange={changeQuery} onAddDevice={() => { result = null; panel = 'choose'; }} onConnect={connect} onOpenWeb={openWeb} {wake} {screenSharing} />
  {/key}
</section>

<Modal open={panel !== null} title={panel === 'invite' ? 'Enroll another device' : 'Add device'} preset="md" dismissible onClose={close}>
  {#if panel === 'invite'}
    <div class="choices">
      <InviteMaker api={link} now={clock.current} />
      <FormButton tone="ghost" onclick={() => (panel = 'choose')}>Back</FormButton>
    </div>
  {:else if panel === 'choose'}
    <div class="choices">
      {#if canEnrolHere}
        <FormButton tone="primary" {busy} onclick={() => void enroll()}>{selfLabel}</FormButton>
      {:else}
        <p>Open the Anima app on the device you want to enroll.</p>
      {/if}
      <FormButton aria-disabled={busy} onclick={() => (panel = 'invite')}>Enroll another device</FormButton>
      {#if result === 'added'}<p role="status">Added to Devices.</p>
      {:else if result === 'already-added'}<p role="status">This device is already in Devices.</p>
      {:else if result === 'not-linked'}<p role="alert">Connect this device to this hub before enrolling it.</p>
      {:else if result === 'failed'}<p role="alert">Couldn’t enroll this device. Try again.</p>
      {/if}
    </div>
  {/if}
</Modal>

<style>
  section { min-inline-size: 0; }
  .choices { display: grid; gap: var(--space-3); justify-items: start; }
  .choices p { margin: 0; }
</style>
