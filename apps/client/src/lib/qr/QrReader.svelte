<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { isMobile } from '$lib/platform/mobile';
  import { qrText } from './read';

  interface Props {
    onread: (text: string) => boolean;
    label?: string;
  }

  const { onread, label = 'Scan QR code' }: Props = $props();
  let open = $state(false);
  let video = $state<HTMLVideoElement>();
  let file = $state<HTMLInputElement>();
  let photo = $state<HTMLInputElement>();
  let active = $state(false);
  let note = $state('');
  let stream: MediaStream | undefined;
  let canvas: HTMLCanvasElement | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let turn = 0;

  function stop(): void {
    turn += 1;
    clearTimeout(timer);
    stream?.getTracks().forEach((track) => track.stop());
    stream = undefined;
    active = false;
  }

  function close(): void {
    stop();
    open = false;
  }

  function received(text: string | null): boolean {
    if (text === null) return false;
    if (!onread(text)) {
      note = 'That QR code is for a different purpose.';
      return false;
    }
    close();
    return true;
  }

  async function frame(current: number): Promise<void> {
    if (turn !== current || video === undefined) return;
    try {
      canvas ??= document.createElement('canvas');
      const text = await qrText(canvas, video, video.videoWidth, video.videoHeight);
      if (turn === current && !received(text)) timer = setTimeout(() => void frame(current), 200);
    } catch {
      if (turn === current) {
        stop();
        note = 'The camera could not read the QR code. Choose an image instead.';
      }
    }
  }

  async function start(): Promise<void> {
    open = true;
    note = '';
    const current = ++turn;
    try {
      const camera = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false });
      if (turn !== current) {
        camera.getTracks().forEach((track) => track.stop());
        return;
      }
      stream = camera;
      active = true;
      await tick();
      if (turn !== current || video === undefined) return;
      video.srcObject = camera;
      await video.play();
      void frame(current);
    } catch {
      if (turn === current) {
        stop();
        note = 'Camera unavailable. Choose an image of the QR code.';
      }
    }
  }

  async function image(event: Event & { currentTarget: HTMLInputElement }): Promise<void> {
    const selected = event.currentTarget.files?.[0];
    event.currentTarget.value = '';
    if (selected === undefined) return;
    stop();
    const current = turn;
    try {
      if (selected.size > 24 * 1024 * 1024) throw new Error('Image too large');
      const bitmap = await createImageBitmap(selected);
      try {
        canvas ??= document.createElement('canvas');
        const text = await qrText(canvas, bitmap, bitmap.width, bitmap.height);
        if (turn === current && !received(text) && text === null) note = 'No QR code was found in that image.';
      } finally {
        bitmap.close();
      }
    } catch {
      if (turn === current) note = 'That image could not be read. Try another photo.';
    }
  }

  onDestroy(stop);
</script>

<FormButton onclick={() => void start()}>{label}</FormButton>
<Modal {open} title={label} preset="md" dismissible onClose={close}>
  <div class="scanner">
    {#if active}
      <!-- The camera preview does not record audio. -->
      <video bind:this={video} muted playsinline aria-label="QR camera preview"></video>
    {/if}
    {#if note}<p role="status">{note}</p>{/if}
    <input type="file" accept="image/*" hidden bind:this={file} onchange={(event) => void image(event)} />
    {#if !active && isMobile(navigator)}
      <input type="file" accept="image/*" capture="environment" hidden bind:this={photo} onchange={(event) => void image(event)} />
      <FormButton onclick={() => photo?.click()}>Take QR photo</FormButton>
    {/if}
    <FormButton onclick={() => file?.click()}>Choose QR image</FormButton>
  </div>
</Modal>

<style>
  .scanner { display: grid; gap: var(--space-3); justify-items: start; }
  .scanner p { margin: 0; }
  video { display: block; inline-size: 100%; max-block-size: calc(var(--space-16) * 6); object-fit: contain; }
</style>
