<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { press } from '$lib/motion/roles/press';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import { actionWords, connectActions, headerLine, type HeaderDevice, type ProtocolAction } from '../header/header';
  import type { SettingsSection } from '../tabs/tabs';

  interface Props {
    device: HeaderDevice;
    /** The button is passed so the session tab grows from it (MOT-005). */
    onConnect: (protocol: SessionProtocol, button: HTMLElement) => void;
    onSettings: (section: SettingsSection | null) => void;
  }

  const { device, onConnect, onSettings }: Props = $props();
  const actions = $derived.by(() => {
    const { main, others } = connectActions(headerLine(device), device.name);
    return [main, ...others];
  });
  const ICONS: Record<SessionProtocol, IconName> = { SSH: 'carbon:terminal', RDP: 'carbon:remote-connection', VNC: 'carbon:screen' };

  function run(action: ProtocolAction, button: HTMLElement): void {
    if (action.section === null) onConnect(action.protocol, button);
    else onSettings(action.section);
  }
</script>

<!-- The ways to connect, docked above the bar in the phone layout (mockup 5): the first one that connects is the
     primary button; one that is not set up opens Settings (DEV-022, MOB-016). -->
<div class="phone-device-connect">
  {#each actions as action, index (action.protocol)}
    <button
      type="button"
      class:primary={index === 0}
      aria-label={action.label}
      onclick={(event) => {
        run(action, event.currentTarget);
      }}
      {@attach press()}><Icon name={action.mode === 'set-up' ? 'carbon:add' : ICONS[action.protocol]} />{actionWords(action)}</button
    >
  {/each}
</div>
