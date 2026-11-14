<script lang="ts">
  import type { Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import DeviceGrid from '$lib/devices/card/DeviceGrid.svelte';
  import DeviceTable from '$lib/devices/card/DeviceTable.svelte';
  import Tally from '$lib/devices/list/Tally.svelte';
  import { tallyOf } from '$lib/devices/list/look';
  import { DEFAULT_QUERY, type ListQuery } from '$lib/devices/query';
  import type { OperatingSystem, Status } from '$lib/hub/generated/graphql';
  import SectionLabel from '$lib/page/SectionLabel.svelte';
  import { SAMPLE } from './sample';

  const MINUTE = 60_000;
  const now = Date.now();

  /** A synthetic device for the specimen: the mockup's twelve, as the hub would send them. */
  function sample(entry: (typeof SAMPLE)[number]): CardDevice {
    const [name, host, os, distribution, status, uptime, readings, protocols, message] = entry;
    const at = new Date(now).toISOString();
    return {
      id: name,
      name,
      host,
      ip: null,
      os: os as OperatingSystem,
      distribution,
      tags: [],
      maintenance: false,
      status: status as Status,
      uptime,
      macAddress: protocols.includes('Wake') ? '00:11:22:33:44:55' : null,
      metricsSource: readings === null ? 'NONE' : 'AGENT',
      readings:
        readings === null
          ? null
          : {
              at,
              cpuPercent: readings[0] ?? null,
              memoryPercent: readings[1] ?? null,
              diskPercent: readings[2] ?? null,
              diskMount: '/',
              otherDrives: 0,
              vramPercent: readings[3] ?? null,
              vramGpu: readings[3] === undefined ? null : { index: 0, name: 'RTX 4070', vendor: 'NVIDIA', hasVram: true },
            },
      accessWarnings: [],
      rdp: { enabled: protocols.includes('RDP') },
      vnc: { enabled: protocols.includes('VNC'), username: null },
      ssh: { enabled: protocols.includes('SSH'), hostKey: protocols.includes('SSH') ? { fingerprintSha256: 'SHA256:specimen' } : null },
      checks: [
        {
          id: `${name}:http`,
          name: 'HTTP check',
          status: status as Status,
          paused: status === 'UNKNOWN',
          heartbeats: { edges: [{ node: { at, status: status as Status, responseTimeMs: 12, message } }] },
        },
      ],
      webInterfaces: [],
    };
  }

  /** The last 50 checks: up, with a recent run of the device's status where it is not up. */
  function beatsOf(device: CardDevice): readonly Beat[] {
    const failing = device.status === 'UP' ? 0 : device.status === 'UNKNOWN' ? 50 : 6;
    return Array.from({ length: 50 }, (_, index) => ({
      checkId: `${device.id}:http`,
      at: new Date(now - (49 - index) * 2 * MINUTE).toISOString(),
      status: index >= 50 - failing ? device.status : 'UP',
    }));
  }

  const devices = SAMPLE.map(sample);
  const beats = new Map(devices.map((device) => [device.id, beatsOf(device)]));
  let query = $state<ListQuery>(DEFAULT_QUERY);
  const shown = $derived(query.statuses.length === 0 ? devices : devices.filter((device) => query.statuses.includes(device.status)));
  const ignore = (): void => undefined;
</script>

<!-- UR-7 Phase E (D18): the Devices list on synthetic devices, so it can be checked without a hub. -->
<section class="devices" aria-labelledby="specimen-devices">
  <SectionLabel text="Devices list" icon="carbon:bare-metal-server" level={3} id="specimen-devices" />
  <Tally tally={tallyOf(devices)} {query} onQueryChange={(next) => (query = next)} />
  <DeviceTable devices={shown} {beats} {query} onConnect={ignore} />
  <DeviceGrid devices={shown.slice(0, 6)} {beats} {query} onConnect={ignore} />
</section>

<style>
  .devices {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-inline-size: 0;
  }
</style>
