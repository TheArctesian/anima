// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { sendArguments } from './arguments';
import { fakeArgumentClient } from './argv.fixture';
import { askedFields } from './fields';

const names = (protocol: 'RDP' | 'VNC', required: string[]) => askedFields(protocol, required)?.map(({ name }) => name);

describe('credential prompt fields (GW-015, K4 §4)', () => {
  it('GW-015: RDP asks username, domain and password in a fixed order, whatever order the gateway used', () => {
    expect(names('RDP', ['password', 'domain', 'username'])).toEqual(['username', 'domain', 'password']);
    expect(names('RDP', ['password'])).toEqual(['password']);
  });

  it('GW-015: VNC asks username and password; only the password is secret', () => {
    const fields = askedFields('VNC', ['username', 'password']);
    expect(fields?.map(({ name, secret }) => [name, secret])).toEqual([
      ['username', false],
      ['password', true],
    ]);
  });

  it.each([
    ['VNC', ['domain']],
    ['VNC', ['password', 'domain']],
    ['RDP', ['username', 'hostname']],
    ['RDP', ['password', '__proto__']],
    ['RDP', ['constructor']],
    ['RDP', []],
  ] as const)('a %s request for %j cannot be satisfied, so nothing is asked', (protocol, required) => {
    expect(askedFields(protocol, required)).toBeUndefined();
  });

  it('a repeated name is asked once', () => {
    expect(names('RDP', ['password', 'password'])).toEqual(['password']);
  });
});

describe('argument value streams (GW-015)', () => {
  it('GW-015: each asked value reaches its own text/plain stream once, with its name, and the stream ends', () => {
    const { client, sent } = fakeArgumentClient();
    const fields = askedFields('RDP', ['username', 'domain', 'password']) ?? [];
    sendArguments(client, fields, { username: 'ana', domain: 'LAB', password: 'pässwörd 🔑' });
    expect(sent()).toEqual([
      { mimetype: 'text/plain', name: 'username', text: 'ana', ends: 1 },
      { mimetype: 'text/plain', name: 'domain', text: 'LAB', ends: 1 },
      { mimetype: 'text/plain', name: 'password', text: 'pässwörd 🔑', ends: 1 },
    ]);
  });

  it('a field left empty is sent empty, and an answer for a name not asked is never sent', () => {
    const { client, sent } = fakeArgumentClient();
    sendArguments(client, askedFields('VNC', ['password']) ?? [], { username: 'not asked' });
    expect(sent()).toEqual([{ mimetype: 'text/plain', name: 'password', text: '', ends: 1 }]);
  });
});
