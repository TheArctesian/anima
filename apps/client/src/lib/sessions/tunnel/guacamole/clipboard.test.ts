// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { CLIPBOARD_LIMIT, receiveClipboard } from './input';
import type { SessionTunnel } from './tunnel';

describe('receiveClipboard (D-9)', () => {
  interface Stream {
    onblob?: (data: string) => void;
    onend?: () => void;
  }
  function fakeClient() {
    const client = { onclipboard: null as null | ((stream: Stream, mimetype: string) => void) };
    return { client, typed: client as unknown as SessionTunnel['client'] };
  }
  const base64 = (text: string) => btoa(String.fromCharCode(...new TextEncoder().encode(text)));

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

  it('D-9: each stream is its own text', () => {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    const first: Stream = {};
    const second: Stream = {};
    client.onclipboard?.(first, 'text/plain');
    client.onclipboard?.(second, 'text/plain');
    first.onblob?.(base64('a'));
    second.onblob?.(base64('b'));
    second.onend?.();
    first.onend?.();
    expect(heard).toEqual(['b', 'a']);
  });
});
