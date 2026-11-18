import { invoke, isTauri } from '@tauri-apps/api/core';
import type { ThisMachine } from '$lib/desktop/this-machine';

/** This computer as a hosted hub lists it (HMD-044), from the shell's `cloud_device`. None of it is secret. */
export interface CloudDevice {
  /** The name of this computer's link device on the hub: what the shell sent at the sign-in. */
  readonly name: string;
  /** The computer name as the OS gives it; empty when the OS gives none. */
  readonly computer: string;
  readonly os: ThisMachine['os'];
}

/** The shell's answer as a `CloudDevice`, or `null`. The answer crosses a process boundary, so every field is checked. */
function cloudDeviceFrom(answer: unknown): CloudDevice | null {
  if (typeof answer !== 'object' || answer === null) return null;
  const { name, computer, os } = answer as Record<string, unknown>;
  if (typeof name !== 'string' || name === '' || typeof computer !== 'string') return null;
  if (os !== 'MACOS' && os !== 'LINUX') return null;
  return { name, computer, os };
}

/** This computer, read by the desktop shell; `null` outside the desktop app and when the shell fails or answers anything else. */
export async function readCloudDevice(): Promise<CloudDevice | null> {
  if (!isTauri()) return null;
  try {
    return cloudDeviceFrom(await invoke('cloud_device'));
  } catch {
    return null;
  }
}
