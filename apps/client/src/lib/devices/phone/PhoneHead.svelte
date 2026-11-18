<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { tallyQuery, type Tally } from '../list/look';
  import type { ListQuery } from '../query';
  import { NAME_DEBOUNCE_MS, debounced } from '../toolbar/toolbar';
  import '$lib/nav/phone/head.css';
  import './list.css';

  interface Props {
    tally: Tally;
    query: ListQuery;
    onQueryChange: (query: ListQuery) => void;
    onAddDevice: () => void;
  }

  const { tally, query, onQueryChange, onAddDevice }: Props = $props();
  const only = $derived(query.statuses.length === 1 ? query.statuses[0] : null);
  let text = $state('');
  const send = debounced(() => {
    onQueryChange({ ...query, name: text });
  }, NAME_DEBOUNCE_MS);

  /** The URL is the truth: Back, Forward and "Clear filters" change the name from outside. */
  $effect(() => {
    text = query.name;
    send.cancel();
  });
  $effect(() => send.cancel);
</script>

<!-- The head of the Devices page in the phone layout (mockup 4): the title, one button to add a device, the counts
     that are also the status filter, and the search. -->
<div class="phone-devices-head">
  <div class="pagehead">
    <h1>Devices</h1>
    <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
    <button type="button" class="phone-head-button" tabindex="0" aria-label="Add device" onclick={onAddDevice} {@attach press()}><Icon name="carbon:add" size="lg" /></button>
  </div>
  <div class="tally" role="group" aria-label="Show devices by status">
    {#each tally.parts as part (part.status)}
      <button type="button" tabindex="0" aria-pressed={only === part.status} onclick={() => { onQueryChange(tallyQuery(query, part.status)); }}>
        <span class="dot {part.status.toLowerCase()}" aria-hidden="true"></span><b>{part.count}</b>{part.words.replace(/^./, (first) => first.toUpperCase())}
      </button>
    {/each}
  </div>
  <label class="phone-search" role="search">
    <span class="label">Search by name</span>
    <Icon name="carbon:search" size="lg" />
    <input
      type="search"
      placeholder="Search"
      value={text}
      oninput={(event) => {
        text = event.currentTarget.value;
        send();
      }}
    />
  </label>
</div>
