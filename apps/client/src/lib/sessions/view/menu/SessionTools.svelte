<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import KeysMenu from './KeysMenu.svelte';
  import { KEY_SEQUENCES, type KeySequence } from './sequences';

  interface Props {
    /** The device's name. */
    name: string;
    /** The session is open; until then the tools are off. */
    ready: boolean;
    finding: boolean;
    filing: boolean;
    /** What the clipboard policy allows, when it limits something (RA-033). */
    policy: string | undefined;
    onkeys: (sequence: KeySequence) => void;
    onpaste: () => void;
    pasteOff: string | undefined;
    onfind: () => void;
    onfiles: () => void;
  }

  const { name, ready, finding, filing, policy, onkeys, onpaste, pasteOff, onfind, onfiles }: Props = $props();
</script>

<!-- The tools of an SSH session's bar on a computer: Keys, Find, Files, the device and the clipboard policy. -->
<KeysMenu disabled={!ready} entries={KEY_SEQUENCES} {onkeys} {onpaste} {pasteOff} />
<button
  type="button"
  class="find"
  aria-expanded={finding}
  disabled={!ready}
  title="Find (Cmd+F, Ctrl+Shift+F)"
  onclick={onfind}
  {@attach press()}
>
  <Icon name="carbon:search" />
  <span>Find</span>
</button>
<button type="button" class="find" aria-expanded={filing} disabled={!ready} title="Files on the device" onclick={onfiles} {@attach press()}><Icon name="carbon:folder" /><span>Files</span></button>
<span class="device">{name}</span>
{#if ready && policy !== undefined}<span class="policy" data-policy title={policy}>{policy}</span>{/if}

<style>
  /* The bar never sets the pane's width: the name and the policy give way first, and the controls keep their size (UI-009). */
  .device,
  .policy {
    min-inline-size: 0;
    overflow: hidden;
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .device {
    font-family: var(--font-mono);
  }

  .policy {
    flex: 1;
    color: var(--text-secondary);
  }

  .find {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    color: var(--text);
    font: inherit;
  }

  .find:disabled {
    color: var(--text-secondary);
  }

  .find:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
