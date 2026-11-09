import { describe, expect, it } from 'vitest';
import { cleanLine, LineCleaner } from '../redaction';
import { BUILDER_TOKEN } from './builds.fixture';

describe('cleanLine (BLD-049, LOG-007)', () => {
  it('BLD-049: removes every known secret, wherever it stands in the line', () => {
    expect(cleanLine(`a ${BUILDER_TOKEN} b ${BUILDER_TOKEN}`, [BUILDER_TOKEN])).toBe('a [redacted] b [redacted]');
  });

  it('BLD-049: a known value shorter than 8 characters is not looked for', () => {
    expect(cleanLine('cargo test a', ['a'])).toBe('cargo test a');
  });

  it.each([
    'https://bucket.s3.example/Anima.AppImage?X-Amz-Algorithm=AWS4&X-Amz-Signature=abc123',
    'https://bucket.example/file?X-Amz-Credential=AKIA%2F20261108',
    'http://hub.example/upload?sig=deadbeef',
    'https://hub.example/upload?a=1&token=secret',
  ])('BLD-049: removes the upload URL %s', (url) => {
    const cleaned = cleanLine(`PUT ${url} done`, []);
    expect(cleaned).toBe('PUT [redacted] done');
  });

  it('BLD-049: keeps a URL that carries no signature', () => {
    expect(cleanLine('fetching https://github.com/TheArctesian/anima.git', [])).toBe('fetching https://github.com/TheArctesian/anima.git');
  });

  it.each([
    ['an encrypted minisign key line', 'RWRTY0IyQ2hlY2tzdW1zZWNyZXQ='],
    ['an unencrypted minisign key line', 'RWQAAEIyAAAAAAAAAAAAAAAAAAAA'],
    ['a Tauri key file in base64', 'dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5'],
  ])('BLD-049: removes %s', (_name, text) => {
    expect(cleanLine(`key ${text} end`, [])).toBe('key [redacted] end');
  });

  it('BLD-049: removes the value of a signing key variable and its password, keeping the name', () => {
    expect(cleanLine('TAURI_SIGNING_PRIVATE_KEY=abcdef', [])).toBe('TAURI_SIGNING_PRIVATE_KEY=[redacted]');
    expect(cleanLine('TAURI_SIGNING_PRIVATE_KEY_PASSWORD: hunter2', [])).toBe('TAURI_SIGNING_PRIVATE_KEY_PASSWORD=[redacted]');
  });

  it('BLD-049: leaves an ordinary build line alone', () => {
    expect(cleanLine('   Compiling anima-desktop v0.1.12', [BUILDER_TOKEN])).toBe('   Compiling anima-desktop v0.1.12');
  });
});

describe('LineCleaner', () => {
  it('BLD-049: removes a private key block whole, from its first line to its last', () => {
    const cleaner = new LineCleaner(() => []);
    const lines = ['before', '-----BEGIN OPENSSH PRIVATE KEY-----', 'b3BlbnNzaC1rZXk=', '-----END OPENSSH PRIVATE KEY-----', 'after'];
    expect(lines.map((line) => cleaner.clean(line))).toEqual(['before', '[redacted]', '[redacted]', '[redacted]', 'after']);
  });

  it('BLD-049: reads the secrets for each line, so a token stored later is still removed', () => {
    let secrets: string[] = [];
    const cleaner = new LineCleaner(() => secrets);
    expect(cleaner.clean(`x ${BUILDER_TOKEN}`)).toBe(`x ${BUILDER_TOKEN}`);
    secrets = [BUILDER_TOKEN];
    expect(cleaner.clean(`x ${BUILDER_TOKEN}`)).toBe('x [redacted]');
  });
});
