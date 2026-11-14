import { describe, expect, it } from 'vitest';
import type { OperatingSystem } from '$lib/hub/generated/graphql';
import { osIcon, osLabel } from './os';

describe('operating system logo', () => {
  it.each<[OperatingSystem, string, string]>([
    ['LINUX', 'simple-icons:linux', 'Linux'],
    ['MACOS', 'simple-icons:apple', 'macOS'],
    ['WINDOWS', 'simple-icons:windows', 'Windows'],
    ['OTHER', 'carbon:screen', 'Other OS'],
  ])('UI-003: %s shows %s and is named %s', (os, icon, label) => {
    expect(osIcon(os)).toBe(icon);
    expect(osLabel(os)).toBe(label);
  });

  it('UI-015: every system has its own logo', () => {
    const systems: OperatingSystem[] = ['LINUX', 'MACOS', 'WINDOWS', 'OTHER'];
    expect(new Set(systems.map((system) => osIcon(system))).size).toBe(systems.length);
  });
});
