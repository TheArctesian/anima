// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { expect, it, vi } from 'vitest';
import PhoneRunner from './PhoneRunner.svelte';

const shell = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke }));

it.each([
  { platform: 'ios', memory: { availableProcessBytes: 134_217_728 }, label: 'Available app memory' },
  { platform: 'android', memory: { availableSystemMemoryBytes: 134_217_728 }, label: 'Available system memory' },
])('OD-14: $platform reads the native report and rechecks it on refresh', async ({ platform, memory, label }) => {
  const report = {
    schemaVersion: 1, platform, simulated: false, osVersion: '27.3',
    observedAt: '2026-11-20T06:00:00Z', charging: 'charging', thermal: 'fair',
    lowPowerMode: false, foreground: true,
    logicalCPUCount: 6, activeCPUCount: 6, physicalMemoryBytes: 8_000_000_000,
    ...memory,
  };
  shell.invoke.mockReset().mockResolvedValueOnce(report).mockResolvedValueOnce({ ...report, charging: 'unplugged' });
  const target = document.createElement('div');
  document.body.append(target);
  const app = mount(PhoneRunner, { target });
  try {
    flushSync();
    const button = target.querySelector('button');
    if (button === null) throw new Error('Missing capability report button');
    expect(shell.invoke).not.toHaveBeenCalled();
    button.click();
    await vi.waitFor(() => expect(target.textContent).toContain('128 MiB'));
    expect(shell.invoke).toHaveBeenCalledWith('runner_capabilities_read');
    expect(target.textContent).toContain(label);
    expect(target.querySelector('details pre')?.textContent).toContain(`"platform": "${platform}"`);
    button.click();
    await vi.waitFor(() => expect(target.textContent).toContain('unplugged'));
    expect(shell.invoke).toHaveBeenCalledTimes(2);
  } finally {
    await unmount(app);
    target.remove();
  }
});
