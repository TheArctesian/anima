// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { expect, it, vi } from 'vitest';
import PhoneRunner from './PhoneRunner.svelte';

const shell = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke }));

it('OD-14: reading capabilities shows the native report and rechecks it on refresh', async () => {
  const report = {
    schemaVersion: 1, platform: 'ios', simulated: false, osVersion: '27.3',
    observedAt: '2026-11-20T06:00:00Z', charging: 'charging', thermal: 'fair',
    lowPowerMode: false, foreground: true,
    logicalCPUCount: 6, activeCPUCount: 6, physicalMemoryBytes: 8_000_000_000,
    availableProcessBytes: 134_217_728,
  };
  shell.invoke.mockResolvedValueOnce(report).mockResolvedValueOnce({ ...report, charging: 'unplugged' });
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
    expect(target.querySelector('details pre')?.textContent).toContain('"platform": "ios"');
    button.click();
    await vi.waitFor(() => expect(target.textContent).toContain('unplugged'));
    expect(shell.invoke).toHaveBeenCalledTimes(2);
  } finally {
    await unmount(app);
    target.remove();
  }
});
