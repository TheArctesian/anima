<script lang="ts">
  import Modal from '$lib/forms/modal/Modal.svelte';
  import AccountStep from './AccountStep.svelte';
  import AuthenticatorStep from './AuthenticatorStep.svelte';

  interface Props {
    /** The hub replaced the account: every token of the operator has stopped (FRN-007). */
    onrecovered: () => void;
    onclose: () => void;
  }

  const { onrecovered, onclose }: Props = $props();
  /** The chosen name and password, until the account is made (LOG-007). */
  let chosen = $state<{ name: string; password: string } | null>(null);

  function made(): void {
    chosen = null;
    onrecovered();
  }
</script>

<!-- The same steps as the first run (FRN-004). When the hub saves, the shell asks the person at this computer in a
     native dialog, which script cannot answer (F-9). -->
<Modal open title="Recover account" preset="md" closeOnScrim={false} onClose={onclose}>
  {#if chosen === null}
    <AccountStep heading="New name and password" onnext={(account) => (chosen = account)} />
  {:else}
    <AuthenticatorStep heading="New authenticator key" name={chosen.name} password={chosen.password} onmade={made} onback={() => (chosen = null)} />
  {/if}
</Modal>
