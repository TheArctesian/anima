import { describe, expect, it } from 'vitest';
import { isValidHost } from './host';

describe('isValidHost', () => {
  it.each([
    '192.168.1.10',
    '0.0.0.0',
    '255.255.255.255',
    '::1',
    '::',
    '[::1]',
    'fe80::1',
    '2001:db8:0:0:0:0:0:1',
    '::ffff:192.168.1.1',
    'nas',
    'nas.home.arpa',
    'a-b.example.com',
    'xn--bcher-kva.example',
    '3com.example',
    '  nas.local  ',
    `${'a'.repeat(63)}.example`,
  ])('DEV-010: accepts %j', (host) => {
    expect(isValidHost(host)).toBe(true);
  });

  it.each([
    '',
    '   ',
    '192.168.1.010',
    '192.168.1.256',
    '192.168.1',
    '192.168.1.1.1',
    '1.2.3.4.5',
    '12345',
    'example.123',
    '-nas.local',
    'nas-.local',
    'nas..local',
    'nas.local.',
    'na s',
    'nas_1.local',
    'http://nas',
    'nas:22',
    '[nas]',
    ':::1',
    '1::2::3',
    '1:2:3:4:5:6:7:8:9',
    '1:2:3:4:5:6:7',
    '12345::1',
    '[::1',
    `${'a'.repeat(64)}.example`,
    `${'a.'.repeat(127)}com`,
  ])('DEV-010: rejects %j', (host) => {
    expect(isValidHost(host)).toBe(false);
  });
});
