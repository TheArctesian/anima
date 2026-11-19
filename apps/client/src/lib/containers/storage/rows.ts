import type { ContainerImage, ContainerNetwork, ContainerStorage, ContainerVolume } from '../api';

// HOST-020: the images, the volumes and the networks as rows of text. A size is the text that Docker writes.

export type StorageKind = 'images' | 'volumes' | 'networks';

export interface StorageTable {
  readonly columns: readonly string[];
  readonly rows: readonly { readonly key: string; readonly cells: readonly string[] }[];
}

/**
 * The containers that use a thing: their names, with how many more Docker counts when the hub found not all of them;
 * the number alone when it found none; "Not used" for zero.
 */
export function usedBy(count: number | null | undefined, names: readonly string[]): string {
  if (names.length === 0) return count == null ? 'Not known' : count === 0 ? 'Not used' : `${String(count)} ${count === 1 ? 'container' : 'containers'}`;
  const more = (count ?? names.length) - names.length;
  return more > 0 ? `${names.join(', ')} and ${String(more)} more` : names.join(', ');
}

/** `repository:tag`; the repository alone for an image with no tag; the first digits of the identifier for one with no name. */
function imageName(image: ContainerImage): string {
  if (image.repository == null) return image.id.replace(/^sha256:/, '').slice(0, 12);
  return image.tag == null ? image.repository : `${image.repository}:${image.tag}`;
}

export function imageRows(images: readonly ContainerImage[]): StorageTable {
  return {
    columns: ['Image', 'Size', 'Only this image', 'Used by'],
    rows: images.map((image) => ({
      key: `${image.id} ${image.repository ?? ''} ${image.tag ?? ''}`,
      cells: [imageName(image), image.size, image.uniqueSize, usedBy(image.containerCount, image.containers)],
    })),
  };
}

export function volumeRows(volumes: readonly ContainerVolume[]): StorageTable {
  return {
    columns: ['Volume', 'Size', 'Driver', 'Used by'],
    rows: volumes.map((volume) => ({ key: volume.name, cells: [volume.name, volume.size, volume.driver, usedBy(volume.containerCount, volume.containers)] })),
  };
}

export function networkRows(networks: readonly ContainerNetwork[]): StorageTable {
  return {
    columns: ['Network', 'Driver', 'Scope', 'Reach', 'Containers'],
    rows: networks.map((network) => ({
      key: network.id,
      cells: [network.name, network.driver, network.scope, network.internal ? 'Inside the device only' : 'Outside too', network.containers.length === 0 ? 'None' : network.containers.join(', ')],
    })),
  };
}

const USAGE_TYPE: Readonly<Record<Exclude<StorageKind, 'networks'>, string>> = { images: 'Images', volumes: 'Local Volumes' };

/** "3 images", "1 volume". */
export function countWords(count: number, kind: StorageKind): string {
  return `${String(count)} ${count === 1 ? kind.slice(0, -1) : kind}`;
}

/** "470.4MB in use, 49.63MB (10%) can be freed by a clean-up" for the type; empty when Docker gave no line for it. */
export function usageLine(usage: ContainerStorage['usage'], kind: Exclude<StorageKind, 'networks'>): string {
  const line = usage.find((each) => each.type === USAGE_TYPE[kind]);
  return line === undefined ? '' : `${line.size} in use, ${line.reclaimable} can be freed by a clean-up`;
}
