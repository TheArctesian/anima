<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Field from '$lib/forms/field/Field.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import type { LinkApi, LinkInvite } from './api';
  import { inviteFailure } from './devices';
  import InviteCode from './InviteCode.svelte';

  interface Props {
    api: Pick<LinkApi, 'invite'>;
    /** The time for the countdown (CQ-021). */
    now: Date;
    /** A new invite was made, for a list that shows the devices. */
    onmade?: () => void;
  }

  const { api, now, onmade }: Props = $props();
  let name = $state('');
  let problem = $state<string | null>(null);
  let busy = $state(false);
  /** The invite on screen, dropped by "Make another" (LOG-007). */
  let made = $state<{ invite: LinkInvite; name: string } | null>(null);

  async function make(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    const chosen = name.trim();
    if (chosen === '' || chosen.length > 64) {
      problem = 'Use 1 to 64 visible characters for the name.';
      return;
    }
    busy = true;
    problem = null;
    const result = await api.invite(chosen);
    busy = false;
    if (!result.ok) {
      problem = inviteFailure(result.error);
      return;
    }
    made = { invite: result.value, name: chosen };
    onmade?.();
  }

  function another(): void {
    made = null;
    name = '';
  }
</script>

{#if made === null}
  <form onsubmit={make} novalidate>
    <Field label="Device name" constraint="The hub lists the device by it." error={problem ?? undefined}>
      {#snippet children(link)}<Input {...link} autocomplete="off" spellcheck="false" maxlength={64} bind:value={name} />{/snippet}
    </Field>
    <FormButton type="submit" tone="primary" busy={busy}>Make invite</FormButton>
  </form>
{:else}
  <div class="made">
    <InviteCode invite={made.invite} name={made.name} {now} />
    <FormButton onclick={another}>Make another</FormButton>
  </div>
{/if}

<style>
  form,
  .made {
    display: grid;
    gap: var(--space-3);
    justify-items: start;
  }

  form :global(.field),
  .made > :global(:first-child) {
    justify-self: stretch;
  }
</style>
