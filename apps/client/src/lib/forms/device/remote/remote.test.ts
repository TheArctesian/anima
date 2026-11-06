import { describe, expect, it } from 'vitest';
import type { PendingDeviceFields } from '../../hub-pending';
import { createInput, updateInput } from '../input';
import { validateDevice } from '../validate';
import { formValues, type DeviceFields, type DeviceFormValues } from '../values';
import { createAdditions, rdpCredentialInput, vncCredentialInput } from './input';
import { isValidMac, normalizedMac } from './values';

const PHASE_1: DeviceFields = {
  name: 'nas',
  host: 'nas.home.arpa',
  kind: 'server',
  os: 'LINUX',
  tags: [],
  ssh: { enabled: true, port: 22, username: 'root', auth: 'PASSWORD' },
};

const K1: PendingDeviceFields = {
  rdp: {
    enabled: true,
    port: 3390,
    username: 'admin',
    domain: 'HOME',
    security: 'NLA_EXT',
    ignoreCertificate: false,
    resizeMethod: 'RECONNECT',
    audioOut: true,
    microphone: false,
    sharedDrive: false,
    credentialStored: true,
    serverCertificate: null,
  },
  vnc: { enabled: true, port: 5901, username: null, credentialStored: true },
  macAddress: 'aa:bb:cc:dd:ee:ff',
  metricsSource: 'SSH',
  transfer: { upload: true, download: false, copy: true, paste: true },
  accessWarnings: [],
};

const SAVED: DeviceFields = { ...PHASE_1, ...K1 };

function edit(base: DeviceFormValues, change: Partial<DeviceFormValues>): DeviceFormValues {
  return { ...base, ...change };
}

function fieldsOf(values: DeviceFormValues): string[] {
  return validateDevice(values).map((error) => error.field);
}

describe('remote values', () => {
  it('RA-030: a device without K1 fields reads as the K1 defaults', () => {
    const values = formValues(PHASE_1);
    expect(values.rdp).toEqual({
      enabled: false,
      port: '3389',
      username: '',
      domain: '',
      security: 'NLA',
      ignoreCertificate: false,
      resizeMethod: 'DISPLAY_UPDATE',
      audioOut: true,
      microphone: false,
      sharedDrive: false,
    });
    expect(values.vnc).toEqual({ enabled: false, port: '5900', username: '' });
    expect(values).toMatchObject({ macAddress: '', metricsSource: 'NONE', transfer: { upload: true, download: true, copy: true, paste: true } });
  });

  it('DEV-011: a saved device fills the settings and never a password', () => {
    const values = formValues(SAVED);
    expect(values.rdp).toMatchObject({ port: '3390', domain: 'HOME', security: 'NLA_EXT', resizeMethod: 'RECONNECT' });
    expect(values.vnc).toEqual({ enabled: true, port: '5901', username: '' });
    expect(values.remoteSecrets).toEqual({ rdpPassword: '', vncPassword: '' });
  });
});

describe('D-2 Phase 1 compatibility', () => {
  const valid = edit(formValues(null), { name: 'nas', host: '10.0.0.5' });

  it('an untouched create form sends exactly the Phase 1 input', () => {
    expect(createInput(valid, 's1')).toEqual({
      siteId: 's1',
      name: 'nas',
      host: '10.0.0.5',
      kind: 'server',
      os: 'OTHER',
      tags: [],
      addDefaultCheck: true,
    });
  });

  it('an untouched update form sends nothing, with or without K1 fields', () => {
    expect(updateInput(formValues(PHASE_1), formValues(PHASE_1))).toEqual({});
    expect(updateInput(formValues(SAVED), formValues(SAVED))).toEqual({});
  });

  it('RA-030: create sends rdp and vnc only when enabled, with the K1 additions that differ', () => {
    const values = edit(valid, {
      rdp: { ...valid.rdp, enabled: true, username: ' admin ', domain: '' },
      vnc: { ...valid.vnc, enabled: true, username: 'me' },
      metricsSource: 'SSH',
    });
    const input = createInput(values, 's1');
    expect(input.rdp).toMatchObject({ enabled: true, port: 3389, username: 'admin', domain: null, security: 'NLA' });
    expect(input.vnc).toEqual({ enabled: true, port: 5900, username: 'me' });
    expect(input.metricsSource).toBe('SSH');
    expect(input).not.toHaveProperty('transfer');
    expect(input).not.toHaveProperty('macAddress');
  });

  it('RA-032, RA-033: create sends the transfer flags only when one is off', () => {
    const off = edit(valid, { transfer: { upload: true, download: false, copy: true, paste: false } });
    expect(createAdditions(off).transfer).toEqual({ upload: true, download: false, copy: true, paste: false });
  });
});

describe('update diffs', () => {
  const before = formValues(SAVED);

  it('RA-030: a changed RDP setting sends RDP whole; VNC stays out', () => {
    const now = edit(before, { rdp: { ...before.rdp, ignoreCertificate: true } });
    const input = updateInput(before, now);
    expect(input.rdp).toMatchObject({ enabled: true, port: 3390, username: 'admin', domain: 'HOME', ignoreCertificate: true });
    expect(input).not.toHaveProperty('vnc');
  });

  it('RA-030: turning RDP off with an unusable port keeps the saved port', () => {
    const now = edit(before, { rdp: { ...before.rdp, enabled: false, port: 'abc' } });
    expect(updateInput(before, now).rdp).toMatchObject({ enabled: false, port: 3390 });
  });

  it('RA-031: a changed MAC address is sent in the stored form; an emptied one sends an empty string', () => {
    expect(updateInput(before, edit(before, { macAddress: 'AA-BB-CC-DD-EE-00' })).macAddress).toBe('aa:bb:cc:dd:ee:00');
    expect(updateInput(before, edit(before, { macAddress: '' })).macAddress).toBe('');
  });

  it('RA-031: the same address in another spelling is no change', () => {
    expect(updateInput(before, edit(before, { macAddress: 'AA-BB-CC-DD-EE-FF' }))).toEqual({});
  });

  it('RA-032: a changed transfer flag and metrics source are sent', () => {
    const now = edit(before, { metricsSource: 'NONE', transfer: { ...before.transfer, download: true } });
    expect(updateInput(before, now)).toEqual({
      metricsSource: 'NONE',
      transfer: { upload: true, download: true, copy: true, paste: true },
    });
  });
});

describe('RA-031 MAC address', () => {
  it.each(['aa:bb:cc:dd:ee:ff', 'AA-BB-CC-DD-EE-FF', 'aA:Bb:cC:Dd:eE:fF', ''])('accepts "%s"', (text) => {
    expect(isValidMac(text)).toBe(true);
  });

  it.each(['aa:bb:cc:dd:ee', 'aa:bb:cc:dd:ee:gg', 'aa:bb-cc:dd-ee:ff', 'aabbccddeeff', 'aa:bb:cc:dd:ee:ff:00'])('rejects "%s"', (text) => {
    expect(isValidMac(text)).toBe(false);
  });

  it('normalizes to lower-case colons', () => {
    expect(normalizedMac(' AA-BB-CC-DD-EE-FF ')).toBe('aa:bb:cc:dd:ee:ff');
  });

  it('reports a bad address on macAddress', () => {
    expect(fieldsOf(edit(formValues(SAVED), { macAddress: 'nope' }))).toEqual(['macAddress']);
  });
});

describe('validation', () => {
  const base = formValues(PHASE_1);

  it('DEV-010: RDP on needs a valid port and a user name; off needs nothing', () => {
    expect(fieldsOf(edit(base, { rdp: { ...base.rdp, port: '0', username: ' ' } }))).toEqual([]);
    const on = edit(base, { rdp: { ...base.rdp, enabled: true, port: '0', username: ' ' } });
    expect(fieldsOf(on)).toEqual(['rdp.port', 'rdp.username']);
  });

  it('DEV-010: VNC on needs a valid port; the user name is optional', () => {
    expect(fieldsOf(edit(base, { vnc: { ...base.vnc, enabled: true, port: 'x' } }))).toEqual(['vnc.port']);
    expect(fieldsOf(edit(base, { vnc: { ...base.vnc, enabled: true } }))).toEqual([]);
  });
});

describe('DEV-011 credentials', () => {
  const base = formValues(SAVED);

  it('send the typed password; empty keeps the stored one', () => {
    expect(rdpCredentialInput(base)).toBeNull();
    expect(vncCredentialInput(base)).toBeNull();
    const typed = edit(base, { remoteSecrets: { rdpPassword: 'r', vncPassword: 'v' } });
    expect(rdpCredentialInput(typed)).toEqual({ password: 'r' });
    expect(vncCredentialInput(typed)).toEqual({ password: 'v' });
  });

  it('are ignored for a protocol that is off', () => {
    const typed = edit(base, {
      rdp: { ...base.rdp, enabled: false },
      vnc: { ...base.vnc, enabled: false },
      remoteSecrets: { rdpPassword: 'r', vncPassword: 'v' },
    });
    expect(rdpCredentialInput(typed)).toBeNull();
    expect(vncCredentialInput(typed)).toBeNull();
  });
});
