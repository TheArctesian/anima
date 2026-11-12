<script lang="ts">
  import { useHub } from '$lib/hub/context';
  import type { HubResult } from '$lib/hub/hub';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { notificationsApi, type NotificationSettings, type NotificationSettingsInput } from './api';
  import ChannelList from './channels/ChannelList.svelte';
  import SettingsForm from './settings/SettingsForm.svelte';
  import { D21_PLACEHOLDERS } from './template/template';

  // The Settings page draws the section and its heading (F-10); this is what goes under the heading. The channels and
  // the settings load apart, so one failing leaves the other working (F-3).
  interface Props {
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { clock }: Props = $props();
  const api = notificationsApi(useHub());
  /** `null` while loading or after a failed load. */
  let settings = $state<NotificationSettings | null>(null);
  let settingsFailed = $state(false);

  async function loadSettings(): Promise<void> {
    const result = await api.settings();
    if (result.ok) {
      settings = result.value;
      settingsFailed = false;
    } else settingsFailed = true;
  }

  $effect(() => {
    void loadSettings();
  });

  async function save(input: NotificationSettingsInput): Promise<HubResult<NotificationSettings>> {
    const result = await api.setSettings(input);
    if (result.ok) settings = result.value;
    return result;
  }
</script>

<ChannelList
  {api}
  defaultTemplate={settings?.template ?? { title: '', body: '' }}
  placeholders={settings?.placeholders ?? D21_PLACEHOLDERS}
  {clock}
/>
<!-- A heading row, inset and ruled like the setting rows around it (SET-001). -->
<h3>Expiry and default template</h3>
<SettingsForm {settings} loadFailed={settingsFailed} onRetry={loadSettings} {save} />

<style>
  /* The heading's line is reserved from tokens, so a font load cannot move what is below it (MOT-007). */
  h3 {
    margin: 0;
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-strong);
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    font-size: var(--text-md);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
  }
</style>
