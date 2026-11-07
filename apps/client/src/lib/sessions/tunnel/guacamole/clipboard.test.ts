// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { CLIPBOARD_LIMIT, receiveClipboard } from './input';
import type { SessionTunnel } from './tunnel';

interface Stream {
  onblob?: (data: string) => void;
  onend?: () => void;
}

function fakeClient() {
  const client = { onclipboard: null as null | ((stream: Stream, mimetype: string) => void) };
  return { client, typed: client as unknown as SessionTunnel['client'] };
}

const base64 = (text: string) => btoa(String.fromCharCode(...new TextEncoder().encode(text)));

describe('receiveClipboard (D-9)', () => {

  it.each([
    ['text/plain', ['abc']],
    ['text/plain;charset=utf-8', ['abc']],
    [' TEXT/Plain ; charset=utf-8', ['abc']],
    ['text/html', []],
    ['text/plainer', []],
    ['', []],
  ])('D-9: a %s stream gives its joined text, other types nothing', (mimetype, expected) => {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    const stream: Stream = {};
    client.onclipboard?.(stream, mimetype);
    stream.onblob?.(base64('ab'));
    stream.onblob?.(base64('c'));
    stream.onend?.();
    expect(heard).toEqual(expected);
  });

  it('D-9: text over the limit is dropped whole', () => {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    const stream: Stream = {};
    client.onclipboard?.(stream, 'text/plain');
    const chunk = base64('x'.repeat(6144));
    for (let sentLength = 0; sentLength <= CLIPBOARD_LIMIT; sentLength += 6144) stream.onblob?.(chunk);
    stream.onend?.();
    expect(heard).toEqual([]);
  });

});

describe('receiveClipboard streams and the limit (D-9)', () => {
  /** A stream on a fresh client, and the texts it delivered. */
  function opened() {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    const stream: Stream = {};
    client.onclipboard?.(stream, 'text/plain');
    return { stream, heard };
  }

  // 'xxx' is 'eHh4' in base64, and the limit minus one is a multiple of three.
  const almostLimit = 'eHh4'.repeat((CLIPBOARD_LIMIT - 1) / 3);

  it('D-9: text of exactly the limit is delivered', () => {
    const { stream, heard } = opened();
    stream.onblob?.(almostLimit);
    stream.onblob?.(base64('y'));
    stream.onend?.();
    expect(heard.map((text) => text.length)).toEqual([CLIPBOARD_LIMIT]);
  });

  it('D-9: parts after the limit was passed are ignored, and the stream still ends quietly', () => {
    const { stream, heard } = opened();
    stream.onblob?.(almostLimit);
    stream.onblob?.(base64('x'));
    stream.onblob?.(base64('y'));
    stream.onblob?.(base64('z'));
    stream.onend?.();
    expect(heard).toEqual([]);
  });

  it('D-9: a stream delivers its text once, even if it is ended twice, and nothing after its end', () => {
    const { stream, heard } = opened();
    stream.onblob?.(base64('once'));
    stream.onend?.();
    stream.onblob?.(base64('late'));
    stream.onend?.();
    expect(heard).toEqual(['once']);
  });

});

describe('receiveClipboard one stream at a time (D-9)', () => {
  it('D-9: one stream is read at a time: a newer one replaces an older one still open, whose text is dropped', () => {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    const streams: Stream[] = [{}, {}, {}];
    for (const [index, stream] of streams.entries()) {
      client.onclipboard?.(stream, 'text/plain');
      stream.onblob?.(base64(String(index)));
    }
    for (const stream of streams) stream.onblob?.(base64('late'));
    streams[2]?.onend?.();
    streams[0]?.onend?.();
    streams[1]?.onend?.();
    expect(heard).toEqual(['2late']);
  });

  it('D-9: a stream that opens after the last one ended is read whole', () => {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    for (const text of ['a', 'b']) {
      const stream: Stream = {};
      client.onclipboard?.(stream, 'text/plain');
      stream.onblob?.(base64(text));
      stream.onend?.();
    }
    expect(heard).toEqual(['a', 'b']);
  });
});
