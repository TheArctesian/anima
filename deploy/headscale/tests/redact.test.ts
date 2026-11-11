// The redaction of captured files (vpn-build.md rule 0.6, LOG-007).
import { describe, expect, it } from 'bun:test';
import { PREAUTH_PLACEHOLDER, redact, RedactError } from '../src/redact.ts';

const real = (prefix: string, digit: string): string => `${prefix}:${digit.repeat(64)}`;
const placeholder = (prefix: string, node: number): string => `${prefix}:${node.toString(16).padStart(64, '0')}`;

describe('redact', () => {
  it('VPN-014: a status file gets node N keys in PublicKey and in the Peer map keys', () => {
    const peer = real('nodekey', 'a');
    const text = JSON.stringify({
      Self: { PublicKey: real('nodekey', 'b'), HostName: 'hub' },
      Peer: { [peer]: { PublicKey: peer, HostName: 'router-home' } },
    });
    const out = JSON.parse(redact(text)) as { Self: { PublicKey: string }; Peer: Record<string, { PublicKey: string }> };
    expect(out.Self.PublicKey).toBe(placeholder('nodekey', 1));
    expect(Object.keys(out.Peer)).toEqual([placeholder('nodekey', 2)]);
    expect(out.Peer[placeholder('nodekey', 2)]?.PublicKey).toBe(placeholder('nodekey', 2));
  });

  it('VPN-014: a node list gets its own mkey, nodekey and discokey, and no pre-auth key survives', () => {
    const text = JSON.stringify([
      {
        name: 'mac',
        machine_key: real('mkey', '1'),
        node_key: real('nodekey', '2'),
        disco_key: real('discokey', '3'),
        pre_auth_key: { id: 3, key: 'hskey-auth-abcdef-secret' },
      },
    ]);
    const out = redact(text);
    expect(out).toContain(placeholder('mkey', 3));
    expect(out).toContain(placeholder('nodekey', 3));
    expect(out).toContain(placeholder('discokey', 3));
    expect(out).toContain(PREAUTH_PLACEHOLDER);
    expect(out).not.toContain('hskey-');
    expect(out).not.toContain('secret');
  });

  it('VPN-014: a key outside a node, or a node with an unknown name, is refused', () => {
    expect(() => redact(JSON.stringify({ stray: real('nodekey', 'c') }))).toThrow(RedactError);
    expect(() => redact(JSON.stringify({ HostName: 'nas', PublicKey: real('nodekey', 'd') }))).toThrow(RedactError);
  });

  it('VPN-014: an auth ID in a URL is refused', () => {
    expect(() => redact(JSON.stringify({ AuthURL: 'http://x/register/hskey-authreq-abc' }))).toThrow(RedactError);
  });
});
