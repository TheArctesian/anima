// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import type { MetricsSource } from '$lib/hub/generated/graphql';
import type { HubError } from '$lib/hub/hub';
import type { StorageDrive } from '../api';
import { drive, mount as mountOf, POP_OS } from './storage.fixture';
import StorageSection from './StorageSection.svelte';

const UNREACHABLE: HubError = { code: 'unreachable', message: 'The hub cannot be reached.' };

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

async function section(drives: readonly StorageDrive[] | null, source: MetricsSource = 'AGENT', error: HubError | null = null): Promise<void> {
  if (app) await unmount(app);
  app = mount(StorageSection, { target: document.body, props: { source, drives, error } });
  flushSync();
}

const rows = () => [...document.querySelectorAll('li')];

it('MET-024: shows the summary line and one row per drive in the order given', async () => {
  await section(POP_OS);
  expect(document.querySelector('h2')?.textContent).toContain('Storage');
  expect(document.querySelector('.summary')?.textContent).toMatch(/^4 drives · .+ · 1 not mounted$/u);
  expect(rows().map((row) => row.querySelector('.name')?.textContent)).toEqual(['nvme0n1', 'sda', 'sdb', 'sdc']);
});

it('MET-024: a row has its kind as text, its mounts in monospace and a meter with its value as text', async () => {
  await section(POP_OS);
  const [system] = rows();
  expect(system?.querySelector('.kind')?.textContent).toBe('NVMe');
  expect([...(system?.querySelectorAll('.path') ?? [])].map((path) => path.textContent)).toEqual(['/', '/boot/efi']);
  const meter = system?.querySelector('[role="meter"]');
  expect(meter?.getAttribute('aria-label')).toBe('/');
  expect(meter?.getAttribute('aria-valuenow')).toBe('81');
  expect(meter?.textContent).toContain('81 %');
  expect(system?.querySelector('.activity')?.textContent).toBe('Read 1.0 MiB/s · Write 512.0 KiB/s · Busy 13 %');
});

it('MET-004: a mount above 85 % is marked degraded in words for a screen reader', async () => {
  await section(POP_OS);
  const meter = rows()[2]?.querySelector('[role="meter"]');
  expect(meter?.getAttribute('aria-valuetext')).toMatch(/, degraded$/u);
});

it('MET-024: a drive not mounted is greyed and says "Not mounted", with no bar', async () => {
  await section(POP_OS);
  const unmounted = rows()[3];
  expect(unmounted?.classList.contains('unmounted')).toBe(true);
  expect(unmounted?.textContent).toContain('Not mounted');
  expect(unmounted?.querySelector('[role="meter"]')).toBeNull();
  expect(unmounted?.querySelector('.track')).toBeNull();
});

it('MET-026: a macOS mount shows its path as the title, with no kind and no activity', async () => {
  await section([drive(null, { kind: null, sizeBytes: null, mounts: [mountOf('/', 40, 4e11, 1e12)] })], 'AGENT');
  const [lone] = rows();
  expect(lone?.querySelector('.name')?.textContent).toBe('/');
  expect(lone?.querySelector('.kind')).toBeNull();
  expect(lone?.querySelector('.activity')).toBeNull();
  expect(lone?.querySelector('.path')?.textContent).toBe('');
});

it('MET-024: says when it waits, when nothing is reported and when metrics failed', async () => {
  await section(null);
  expect(document.body.textContent).toContain('Loading storage');
  await section([]);
  expect(document.body.textContent).toContain('No drives reported yet');
  await section(POP_OS, 'SSH', UNREACHABLE);
  expect(document.body.textContent).toContain('not available');
  expect(rows()).toHaveLength(0);
});

it('MET-024: hidden when metrics are off; the Metrics section says why', async () => {
  await section(POP_OS, 'NONE');
  expect(document.querySelector('section')).toBeNull();
});
