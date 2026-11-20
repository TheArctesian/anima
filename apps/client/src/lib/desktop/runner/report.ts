import { invoke } from '@tauri-apps/api/core';

export interface RunnerReport {
  readonly schemaVersion: 1;
  readonly platform: 'ios' | 'android';
  readonly simulated: boolean;
  readonly osVersion: string;
  readonly observedAt: string;
  readonly charging: string;
  readonly thermal: string;
  readonly lowPowerMode: boolean;
  readonly foreground: boolean;
  readonly logicalCPUCount: number;
  readonly activeCPUCount: number;
  readonly physicalMemoryBytes: number;
  readonly availableProcessBytes?: number;
  readonly availableSystemMemoryBytes?: number;
  readonly availableStorageBytes?: number;
}

function isRunnerReport(value: unknown): value is RunnerReport {
  if (typeof value !== 'object' || value === null) return false;
  const report = value as Record<string, unknown>;
  return report.schemaVersion === 1
    && (report.platform === 'ios' || report.platform === 'android')
    && [report.osVersion, report.observedAt, report.charging, report.thermal].every((field) => typeof field === 'string')
    && [report.foreground, report.lowPowerMode, report.simulated].every((field) => typeof field === 'boolean')
    && [report.logicalCPUCount, report.activeCPUCount, report.physicalMemoryBytes].every((field) => typeof field === 'number')
    && [report.availableProcessBytes, report.availableSystemMemoryBytes, report.availableStorageBytes].every((field) => field === undefined || typeof field === 'number');
}

export async function readRunnerReport(): Promise<RunnerReport> {
  const report: unknown = await invoke('runner_capabilities_read');
  if (!isRunnerReport(report)) throw new Error('The runner returned an invalid capability report.');
  return report;
}

export function formatRunnerBytes(bytes: number | undefined): string {
  return bytes === undefined ? 'Unavailable' : `${Math.round(bytes / 1_048_576).toLocaleString()} MiB`;
}
