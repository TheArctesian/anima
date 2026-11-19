import { expect, it } from 'vitest';
import { NETWORKS, STORAGE } from '../containers.fixture';
import { imageRows, networkRows, usageLine, usedBy, volumeRows } from './rows';

it('HOST-020: each image has its size, the part only it uses, and the containers that use it', () => {
  const table = imageRows(STORAGE.images);
  expect(table.columns).toEqual(['Image', 'Size', 'Only this image', 'Used by']);
  // Docker counts three containers and the hub found the name of one.
  expect(table.rows[0]?.cells).toEqual(['example/tracks:0.36', '470MB', '49.49MB', 'tracks-app-1 and 2 more']);
  // An image with no name shows the first digits of its identifier.
  expect(table.rows[1]?.cells).toEqual(['f59abe656b69', '13.6MB', '13.6MB', 'Not used']);
});

it('HOST-020: each volume has its size and the containers that mount it, and each network its containers', () => {
  expect(volumeRows(STORAGE.volumes).rows.map((row) => row.cells)).toEqual([
    ['tracks-data', '232.9MB', 'local', 'tracks-app-1'],
    ['old-data', '1.2GB', 'local', 'Not used'],
  ]);
  expect(networkRows(NETWORKS).rows.map((row) => row.cells)).toEqual([
    ['tracks_default', 'bridge', 'local', 'Outside too', 'tracks-app-1, tracks-worker-1'],
    ['backend', 'bridge', 'local', 'Inside the device only', 'None'],
  ]);
});

it('HOST-020: the containers that use a thing are names, a number, or not known', () => {
  expect(usedBy(2, [])).toBe('2 containers');
  expect(usedBy(1, [])).toBe('1 container');
  expect(usedBy(null, [])).toBe('Not known');
  expect(usedBy(null, ['a'])).toBe('a');
  expect(usedBy(1, ['a', 'b'])).toBe('a, b');
});

it('HOST-020: the space a clean-up can free is what Docker says for the type', () => {
  expect(usageLine(STORAGE.usage, 'images')).toBe('483.6MB in use, 13.6MB (2%) can be freed by a clean-up');
  expect(usageLine(STORAGE.usage, 'volumes')).toBe('1.433GB in use, 1.2GB (83%) can be freed by a clean-up');
  expect(usageLine([], 'images')).toBe('');
});
