import { expect, it } from 'vitest';
import type { LiveSession, SessionChange } from './api';
import { applyChange, newestFirst } from './list';

const session = (id: string, startedAt: string): LiveSession => ({
  id,
  deviceId: 'device-1',
  deviceName: 'desk',
  protocol: 'SSH',
  operatorName: 'ada',
  startedAt,
  clientAddr: '100.64.0.2',
});
const change = (live: LiveSession, endedAt: string | null): SessionChange => ({ ...live, endedAt, endReason: endedAt === null ? null : 'OPERATOR' });

const OLD = session('a', '2026-11-04T10:00:00Z');
const NEW = session('b', '2026-11-04T11:00:00Z');

it('RA-034: a started session is added and the list is newest first', () => {
  expect(applyChange([OLD], change(NEW, null)).map((live) => live.id)).toEqual(['b', 'a']);
});

it('RA-034: a change for a session already listed replaces it', () => {
  const renamed = { ...OLD, deviceName: 'laptop' };
  const next = applyChange([NEW, OLD], change(renamed, null));
  expect(next.map((live) => live.deviceName)).toEqual(['desk', 'laptop']);
});

it('RA-034: an ended session leaves the list', () => {
  expect(applyChange([NEW, OLD], change(NEW, '2026-11-04T12:00:00Z'))).toEqual([OLD]);
});

it('RA-034: the end of an unknown session changes nothing', () => {
  const list = [OLD];
  expect(applyChange(list, change(NEW, '2026-11-04T12:00:00Z'))).toBe(list);
});

it('RA-034: a running change carries no end fields into the list', () => {
  expect(applyChange([], change(OLD, null))[0]).toEqual(OLD);
});

it('RA-034: newestFirst leaves its input alone', () => {
  const list = [OLD, NEW];
  newestFirst(list);
  expect(list.map((live) => live.id)).toEqual(['a', 'b']);
});
