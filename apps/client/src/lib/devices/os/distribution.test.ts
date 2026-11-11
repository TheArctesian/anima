import { expect, it } from 'vitest';
import { distribution } from './distribution';

it('Pop!_OS, Ubuntu and Arch each get their own logo and name from the os-release ID', () => {
  expect(distribution('pop')).toEqual({ icon: 'nf-linux-pop_os', label: 'Pop!_OS' });
  expect(distribution('ubuntu')?.icon).toBe('nf-linux-ubuntu');
  expect(distribution('arch')?.label).toBe('Arch Linux');
  expect(distribution('opensuse-tumbleweed')?.icon).toBe('nf-linux-tumbleweed');
});

it('an unknown, missing or inherited ID has no logo of its own, so the plain Linux logo stays', () => {
  expect(distribution('someos')).toBeNull();
  expect(distribution(null)).toBeNull();
  expect(distribution(undefined)).toBeNull();
  expect(distribution('constructor')).toBeNull();
  expect(distribution('toString')).toBeNull();
});
