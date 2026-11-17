<script lang="ts">
  import { resolve } from '$app/paths';
  import type { DeviceCheck } from '$lib/checks/api';
  import HeartbeatBar from '$lib/checks/HeartbeatBar.svelte';
  import { uptimeText, type Beat } from '$lib/checks/heartbeats';
  import type { DeviceDetail } from '$lib/devices/api';
  import { statusLabel } from '$lib/devices/card/card';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { statusReason } from '../header/header';
  import { responseText } from './phone';

  interface Props {
    device: DeviceDetail;
    checks: readonly DeviceCheck[] | null;
    beats: readonly Beat[] | null;
    /** Opens the Settings tab (DEV-022). */
    onSettings: () => void;
  }

  const { device, checks, beats, onSettings }: Props = $props();
  const reason = $derived(statusReason(device, checks));
  let copied = $state(false);

  async function copyHost(): Promise<void> {
    await navigator.clipboard.writeText(device.host);
    copied = true;
  }
</script>

<!-- The device page's head in the phone layout (mockup 5): back and more, the name on two lines at most, the status and
     why, the address with a copy button, three figures and the last 50 checks (MOB-015). -->
<header class="phone-device-head">
  <div class="navrow">
    <a class="back" href={resolve('/devices')} {@attach press()}><Icon name="carbon:chevron-left" />Devices</a>
    <button type="button" class="iconbtn" aria-label="Settings of {device.name}" onclick={onSettings} {@attach press()}><Icon name="carbon:overflow-menu-vertical" /></button>
  </div>
  <h1 class="name">{device.name}</h1>
  <p class="status" aria-live="polite">
    {#if device.status === 'UP'}
      <span class="dot up" aria-hidden="true"></span>{statusLabel(device.status)}
    {:else}
      <span class="badge {device.status.toLowerCase()}">{statusLabel(device.status)}</span>
    {/if}
    {#if reason !== null}<span class="why">{reason}</span>{/if}
  </p>
  <p class="host">
    <span class="mono">{device.host}</span>
    <button type="button" class="iconbtn" aria-label={copied ? 'Address copied' : 'Copy address'} onclick={() => void copyHost()} {@attach press()}
      ><Icon name="carbon:copy" size="sm" /></button
    >
  </p>
  <dl class="figs">
    <div><dt>Uptime 24 h</dt><dd>{uptimeText(device.day ?? null)}</dd></div>
    <div><dt>30 d</dt><dd>{uptimeText(device.month ?? null)}</dd></div>
    <div><dt>Response</dt><dd>{responseText(beats ?? [])}</dd></div>
  </dl>
  <p class="beatcap" aria-hidden="true"><span>Last 50 checks</span><span>now</span></p>
  <HeartbeatBar beats={beats ?? []} />
</header>
