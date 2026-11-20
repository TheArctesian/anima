<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import type { SettingsSection } from '../../tabs/tabs';
  import '../geometry.css';
  import { waysIn, type SideDevice, type WayIn } from './side';

  interface Props {
    device: SideDevice;
    /** The button is passed so the session tab grows from it (MOT-005). */
    onConnect: (protocol: SessionProtocol, button: HTMLElement) => void;
    /** Missing where the page cannot open a web interface: its rows then have no button. */
    onOpenWeb?: ((webInterface: WebInterface, button: HTMLElement) => void) | undefined;
    onSettings: (section: SettingsSection | null) => void;
  }

  const { device, onConnect, onOpenWeb, onSettings }: Props = $props();
  const ways = $derived(waysIn(device, onOpenWeb !== undefined));

  function run(way: WayIn, button: HTMLElement): void {
    if (way.webInterface !== null) onOpenWeb?.(way.webInterface, button);
    else if (way.action?.section == null) onConnect(way.action?.protocol ?? 'SSH', button);
    else onSettings(way.action.section);
  }
</script>

<!-- Each way in to the device as a row: its name, one line of detail and its button (R-224, DEV-022, WEB-001). -->
<section class="overview-card connect" aria-labelledby="overview-connect">
  <h2 id="overview-connect" class="overview-card-title">Connect</h2>
  <ul>
    {#each ways as way (way.key)}
      <li>
        <span class="way"><span class="name">{way.name}</span><span class="detail" title={way.detail}>{way.detail}</span></span>
        {#if way.button !== null}
          <FormButton
            size="sm"
            tone={way.main ? 'primary' : way.action?.mode === 'set-up' ? 'ghost' : 'secondary'}
            aria-label={way.action?.label ?? `Open ${way.name}`}
            onclick={(event) => { run(way, event.currentTarget); }}>{way.button}</FormButton>
        {/if}
      </li>
    {/each}
  </ul>
</section>

<style>
  ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-block-size: var(--space-12);
    padding-block: var(--space-2);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .way {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  .name {
    color: var(--text);
    font-weight: var(--weight-medium);
  }

  /* Machine text: a login, a port, a scheme. A long one is cut, and its full text is the tooltip. */
  .detail {
    overflow: hidden;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
