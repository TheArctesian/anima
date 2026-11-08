import { describe, expect, it } from 'vitest';
import { siteFieldErrors, suggestedSite, validateNewSite, VPN_SITE } from './site';

describe('suggestedSite', () => {
  it.each([
    ['100.64.0.2', VPN_SITE],
    ['100.127.255.254', VPN_SITE],
    ['10.10.0.5', { name: 'Local network', subnet: '10.10.0.0/24' }],
    ['100.128.0.1', { name: 'Local network', subnet: '100.128.0.0/24' }],
    ['nas.home.arpa', { name: '', subnet: '' }],
    ['fd00::1', { name: '', subnet: '' }],
  ])('VPN-002: offers a site for %s', (host, site) => {
    expect(suggestedSite(host)).toEqual(site);
  });
});

describe('validateNewSite', () => {
  it('VPN-001: accepts the VPN site and an IPv6 block, which the hub checks', () => {
    expect(validateNewSite(VPN_SITE)).toEqual([]);
    expect(validateNewSite({ name: 'Lab', subnet: 'fd00::/64' })).toEqual([]);
  });

  it.each([
    ['', '10.0.0.0/24', 'site.name'],
    ['Lab', '10.0.0.0', 'site.subnet'],
    ['Lab', '10.0.0.0/33', 'site.subnet'],
    ['Lab', '10.0.0.5/24', 'site.subnet'],
    ['Lab', '10.0.0/24', 'site.subnet'],
  ])('VPN-001: refuses name %j with subnet %j', (name, subnet, field) => {
    expect(validateNewSite({ name, subnet }).map((error) => error.field)).toEqual([field]);
  });
});

it('a hub error about the site moves under site.', () => {
  expect(siteFieldErrors([{ field: 'name', message: 'Too long.' }])).toEqual([{ field: 'site.name', message: 'Too long.' }]);
});
