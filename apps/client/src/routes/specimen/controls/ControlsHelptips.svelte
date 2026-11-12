<script lang="ts">
  import HelpTip from '$lib/overlay/helptip/HelpTip.svelte';
</script>

<section aria-labelledby="specimen-helptips">
  <h3 id="specimen-helptips">HelpTip</h3>
  <div class="row">
    <span>Short text <HelpTip label="retention">Check results are kept for 30 days by default.</HelpTip></span>
    <span>
      Fingerprint
      <HelpTip label="host key">
        <p>The key the device presented the first time.</p>
        <p><code>SHA256:3f9aG0n+6hQ2xT1uVwZ8cK5dL7oPbR4yEiNsJmAq0Hk</code></p>
      </HelpTip>
    </span>
    <span>
      List
      <HelpTip label="allowed ports">
        <p>Allowed ports:</p>
        <ul>
          <li>22 for SSH</li>
          <li>3389 for RDP</li>
          <li>5900 for VNC</li>
        </ul>
      </HelpTip>
    </span>
    <span class="end">Near the right edge, it moves inside the window <HelpTip label="placement">This panel is wider than the room to the right, so it moves left.</HelpTip></span>
  </div>
</section>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }

  h3 {
    margin: 0;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-6);
  }

  .end {
    margin-inline-start: auto;
  }
</style>
