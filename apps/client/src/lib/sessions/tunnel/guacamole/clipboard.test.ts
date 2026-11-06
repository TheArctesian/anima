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
    ['text/html', []],
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
