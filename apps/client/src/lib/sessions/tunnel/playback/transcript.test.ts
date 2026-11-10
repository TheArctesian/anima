import { describe, expect, it } from 'vitest';
import { guacamoleStream } from '$lib/fixtures/phase3';
import { openTranscript } from './transcript';

const encoder = new TextEncoder();

/** `text` as UTF-8, sent in frames of `size` bytes (a frame may end inside a character). */
function received(text: string, size: number) {
  const transcript = openTranscript();
  const bytes = encoder.encode(text);
  const accepted: boolean[] = [];
  for (let at = 0; at < bytes.length; at += size) accepted.push(transcript.receive(bytes.subarray(at, at + size)));
  return { transcript, accepted };
}

describe('the Guacamole transcript (K4 §11, §12, RA-041)', () => {
  it('RA-041: the K6 stream comes out unchanged, whatever the frame size', async () => {
    for (const size of [1, 2, 7, 64, 65_536]) {
      const { transcript, accepted } = received(guacamoleStream.text, size);
      expect(accepted.every(Boolean)).toBe(true);
      expect(await transcript.blob().text()).toBe(guacamoleStream.text);
    }
  });

  it('K4 §12: frames that cut a character outside the BMP still join; an instruction holding one is dropped (the 1.5.5 player needs ASCII)', async () => {
    const { transcript, accepted } = received('4.sync,1.0;4.name,2.😀a;4.sync,1.5;', 1);
    expect(accepted.every(Boolean)).toBe(true);
    expect(transcript.unreadable()).toBe(false);
    expect(await transcript.blob().text()).toBe('4.sync,1.0;4.sync,1.5;');
  });

  it('K4 §12: the internal opcode, nest and audio are dropped; drawing, sync and the rest are kept', async () => {
    const text = '0.,36.00000000-0000-4000-8000-000000000000;4.nest,1.0,11.4.sync,1.9;;5.audio,1.1,9.audio/L16;4.blob,1.1,4.AAAA;4.sync,1.0;';
    const { transcript } = received(text, 5);
    expect(await transcript.blob().text()).toBe('4.blob,1.1,4.AAAA;4.sync,1.0;');
  });

  it('K4 §12: lengths are written again in canonical form', async () => {
    const { transcript } = received('04.sync,01.0;', 3);
    expect(await transcript.blob().text()).toBe('4.sync,1.0;');
  });

  it('K4 §11: an incomplete recording cut inside an instruction gives the whole instructions before the cut', async () => {
    const { transcript } = received(`${guacamoleStream.text}4.rect,1.0,3.10`, 4);
    expect(transcript.unreadable()).toBe(false);
    expect(await transcript.blob().text()).toBe(guacamoleStream.text);
  });

  it('bytes that are not UTF-8 make it unreadable, keep what came before, and refuse every later frame', async () => {
    const transcript = openTranscript();
    expect(transcript.receive(encoder.encode('4.sync,1.0;'))).toBe(true);
    expect(transcript.receive(Uint8Array.from([0xff, 0xfe]))).toBe(false);
    expect(transcript.receive(encoder.encode('4.sync,1.1;'))).toBe(false);
    expect(transcript.unreadable()).toBe(true);
    expect(await transcript.blob().text()).toBe('4.sync,1.0;');
  });

  it('text that is not instructions makes it unreadable', () => {
    const { transcript, accepted } = received('{"version":2}\n', 4);
    expect(accepted.at(-1)).toBe(false);
    expect(transcript.unreadable()).toBe(true);
  });
});
