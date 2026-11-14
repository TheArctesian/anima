<script lang="ts">
  import type { DeviceDetail } from '$lib/devices/api';
  import type { HubResult } from '$lib/hub/hub';
  import SectionLabel from '$lib/page/SectionLabel.svelte';
  import { saveDevice } from '../../devices/[id]/save';
  import type { SettingsCalls } from '../../devices/[id]/settings/calls';
  import SettingsPanel from '../../devices/[id]/settings/SettingsPanel.svelte';
  import { SAMPLE_DEVICE, updated } from './settings-sample';

  /** How long the fake hub takes, so "Saving…" can be seen before "Saved". */
  const LATENCY = 450;
  let device = $state<DeviceDetail>(SAMPLE_DEVICE);

  /** Answers after the fake hub's latency. */
  function later<T>(answer: () => HubResult<T>): Promise<HubResult<T>> {
    return new Promise((done) => setTimeout(() => { done(answer()); }, LATENCY));
  }

  const stored = (credentialStored: boolean) => () => ({ ok: true as const, value: { credentialStored } });
  const unavailable = (): HubResult<never> => ({ ok: false, error: { code: 'UNAVAILABLE', message: 'The specimen has no hub.' } });

  // A synthetic hub: each save changes the device in memory, and the name "atlas" is refused.
  const calls: SettingsCalls = {
    onSave: async (values) => {
      const saved = await saveDevice(device, values, {
        update: (_id, input) => later(() => updated(device, input)),
        setSshCredential: () => later(stored(true)),
        setRdpCredential: () => later(stored(true)),
        setVncCredential: () => later(stored(true)),
      });
      device = saved.device;
      return saved.result;
    },
    onAcceptHostKey: () => later(unavailable),
    onAcceptServerCertificate: () => later(unavailable),
    onClearCredential: () => later(() => { device = { ...device, ssh: { ...device.ssh, credentialStored: false } }; return stored(false)(); }),
    onClearRdpCredential: () => later(() => { device = { ...device, rdp: { ...device.rdp, credentialStored: false } }; return stored(false)(); }),
    onClearVncCredential: () => later(() => { device = { ...device, vnc: { ...device.vnc, credentialStored: false } }; return stored(false)(); }),
    webInterfaces: { onCreate: () => later(unavailable), onUpdate: () => later(unavailable), onDelete: () => later(unavailable), onAcceptSuggestion: () => later(unavailable), onDismissSuggestion: () => later(unavailable) },
  };
  const agentCalls = { agentStatus: () => later(unavailable), enrolAgent: () => later(unavailable), revokeAgentToken: () => later(unavailable) };
  const ignore = (): void => undefined;
</script>

<!-- UR-7 (D17): the device's Settings tab on a synthetic device, so saving on change can be checked without a hub. -->
<section class="lane" aria-labelledby="specimen-device-settings">
  <SectionLabel text="Device settings · try the name atlas to see a refusal" icon="carbon:settings-adjust" level={3} id="specimen-device-settings" />
  <SettingsPanel
    {device}
    section={null}
    {calls}
    agent={{ calls: agentCalls, origin: 'https://hub.specimen', reveal: false, shell: null }}
    now={() => new Date()}
    onTest={() => later(unavailable)}
    onOpenChecks={ignore}
    onDelete={() => later(unavailable)}
  />
</section>

<style>
  .lane {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-inline-size: 0;
  }
</style>
