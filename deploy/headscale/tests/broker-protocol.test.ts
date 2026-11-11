// The broker's request parser (R-055, VP-7, VP-10): only the six verbs with valid values parse.
import { describe, expect, it } from 'bun:test';
import { MAX_REQUEST_BYTES, parseRequest } from '../broker/protocol.ts';

const AUTH_ID = 'hskey-authreq-Ab3_-Zz9Ab3_-Zz9Ab3_-Zz9';

describe('parseRequest', () => {
  it('VP-7: only the five verbs parse (and renew, VP-10)', () => {
    expect(parseRequest('{"verb":"nodes"}')).toEqual({ verb: 'nodes' });
    expect(parseRequest('{"verb":"personal-key"}')).toEqual({ verb: 'personal-key' });
    expect(parseRequest('{"verb":"router-key","site":"home"}')).toEqual({ verb: 'router-key', site: 'home' });
    expect(parseRequest('{"site":"office-1","verb":"approve-route","node":7}')).toEqual({ verb: 'approve-route', node: 7, site: 'office-1' });
    expect(parseRequest(JSON.stringify({ verb: 'register', authId: AUTH_ID }))).toEqual({ verb: 'register', authId: AUTH_ID });
    expect(parseRequest('{"verb":"renew","node":6}')).toEqual({ verb: 'renew', node: 6 });
    for (const verb of ['open-device', 'close-device', 'delete', 'NODES', 'toString', 'constructor', '']) {
      expect([verb, parseRequest(JSON.stringify({ verb }))]).toEqual([verb, 'bad-request']);
    }
  });

  it('VP-7: the auth ID pattern matches the IDs Headscale 0.29.7 makes (fact 11)', () => {
    expect(parseRequest(JSON.stringify({ verb: 'register', authId: 'hskey-authreq-AAAAAAAAAAAAAAAAAAAAAAAA' }))).not.toBe('bad-request');
    expect(parseRequest(JSON.stringify({ verb: 'register', authId: 'hskey-authreq-AAAAAAAAAAAAAAAAAAAAAAA' }))).toBe('bad-request');
    expect(parseRequest(JSON.stringify({ verb: 'register', authId: 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA' }))).toBe('bad-request');
  });

});

describe('parseRequest refusals', () => {
  const nodes = '{"verb":"nodes"}';
  const long = nodes.padEnd(MAX_REQUEST_BYTES + 1, ' ');
  it.each([
    ['a site longer than 32', '{"verb":"router-key","site":"abcdefghijklmnopqrstuvwxyz0123456"}'],
    ['an upper-case site', '{"verb":"router-key","site":"Home"}'],
    ['a site with ../', '{"verb":"router-key","site":"../etc"}'],
    ['an empty site', '{"verb":"router-key","site":""}'],
    ['a negative node', '{"verb":"approve-route","node":-1,"site":"home"}'],
    ['a float node', '{"verb":"approve-route","node":1.5,"site":"home"}'],
    ['node 2^31', `{"verb":"renew","node":${String(2 ** 31)}}`],
    ['node 0', '{"verb":"renew","node":0}'],
    ['a node as a string', '{"verb":"renew","node":"6"}'],
    ['an auth ID with a space', JSON.stringify({ verb: 'register', authId: 'hskey-authreq-Ab3_-Zz9Ab3_ Zz9Ab3_-Zz9' })],
    ['an auth ID starting with -', JSON.stringify({ verb: 'register', authId: '-hskey-authreq-Ab3_-Zz9Ab3_-Zz9Ab3_-Zz' })],
    ['an auth ID that is a flag', JSON.stringify({ verb: 'register', authId: '--user=intruder' })],
    ['an extra key', '{"verb":"nodes","site":"home"}'],
    ['a __proto__ key', '{"verb":"nodes","__proto__":{"site":"home"}}'],
    ['a missing value', '{"verb":"router-key"}'],
    ['a 1 KiB + 1 line', long],
    ['an array', '[{"verb":"nodes"}]'],
    ['not JSON', 'verb=nodes'],
    ['null', 'null'],
    ['a duplicate verb that hides a refused one', '{"verb":"open-device","verb":"nodes","node":1}'],
  ])('VP-7: refuses %s', (_name, line) => {
    expect(parseRequest(line)).toBe('bad-request');
  });

  it('VP-7: a line of exactly 1 KiB still parses', () => {
    expect(parseRequest(nodes.padEnd(MAX_REQUEST_BYTES, ' '))).toEqual({ verb: 'nodes' });
  });

  it('VP-7: the size limit counts UTF-8 bytes, not characters', () => {
    const padding = 'é'.repeat(MAX_REQUEST_BYTES / 2);
    expect(parseRequest(`{"verb":"nodes","x":"${padding}"}`)).toBe('bad-request');
    expect(new TextEncoder().encode(padding).length).toBe(MAX_REQUEST_BYTES);
  });

  it('VP-7: random lines never throw and only parse to a known verb (property)', () => {
    const pieces = ['{', '}', '"verb"', ':', '"nodes"', '"renew"', '"node"', '1', ',', '"site"', '"home"', '[', ']', 'null', '"__proto__"', ' '];
    let seed = 7;
    const next = (): number => {
      seed = (seed * 1_103_515_245 + 12_345) % 2 ** 31;
      return seed;
    };
    for (let run = 0; run < 2000; run += 1) {
      const line = Array.from({ length: next() % 12 }, () => pieces[next() % pieces.length]).join('');
      const parsed = parseRequest(line);
      if (typeof parsed !== 'string') expect(['nodes', 'renew']).toContain(parsed.verb);
    }
  });
});
