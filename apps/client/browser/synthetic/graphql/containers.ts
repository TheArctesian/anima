import { GraphQLError } from 'graphql';
import type { Row } from '../fixtures';
import type { HubState } from '../model';

// The containers of a device, so the Containers tab has something to show (HOST-001, HOST-013). `nas` runs one Compose
// project and one container with none; `gpu-box` runs one container that is not healthy. Every other device answers as a
// device with no docker command does. The same containers give the details, the images, volumes and networks, and the
// answer for a newer image (HOST-017 to HOST-021); no request leaves the browser. Every image and name is an example one (CON-003).

const ID_DIGITS = '0123456789abcdef';

/** A 64-digit identifier made from a seed, so each container has the same one at every read. */
function identifier(seed: number): string {
  return Array.from({ length: 64 }, (_, index) => ID_DIGITS[(seed * 7 + index * 13) % 16]).join('');
}

function container(seed: number, change: Row): Row {
  return { id: identifier(seed), image: 'example/app:1', state: 'RUNNING', status: 'Up 3 hours', health: 'NONE', ports: [], createdAt: '2026-11-18T08:00:00Z', composeProject: null, composeService: null, composeProjectState: null, ...change };
}

const BY_DEVICE: Readonly<Record<string, readonly Row[]>> = {
  nas: [
    container(1, { name: 'tracks-app-1', image: 'example/tracks:0.36', status: 'Up 3 hours (healthy)', health: 'HEALTHY', ports: ['0.0.0.0:3000->3000/tcp'], composeProject: 'tracks', composeService: 'app', composeProjectState: 'PARTLY_RUNNING' }),
    container(2, { name: 'tracks-db-1', image: 'postgres:17', status: 'Up 3 hours (healthy)', health: 'HEALTHY', ports: ['5432/tcp'], composeProject: 'tracks', composeService: 'db', composeProjectState: 'PARTLY_RUNNING' }),
    container(3, { name: 'tracks-worker-1', image: 'example/tracks:0.36', state: 'EXITED', status: 'Exited (1) 2 hours ago', composeProject: 'tracks', composeService: 'worker', composeProjectState: 'PARTLY_RUNNING' }),
    container(4, { name: 'recorder', image: 'example/recorder:1.0', status: 'Up 9 days', ports: ['0.0.0.0:8083->8083/tcp'] }),
  ],
  'gpu-box': [container(5, { name: 'ollama', image: 'example/ollama:0.9', status: 'Up 2 minutes (unhealthy)', health: 'UNHEALTHY', ports: ['127.0.0.1:11434->11434/tcp'] })],
};

function refuse(code: string, message: string): never {
  throw new GraphQLError(message, { extensions: { code } });
}

function containersOf(state: HubState, deviceId: string): readonly Row[] {
  const device = state.fixtures.devices.find((found) => found.id === deviceId);
  if (device === undefined) refuse('NOT_FOUND', 'No device has this ID.');
  const found = BY_DEVICE[String(device.name)];
  if (found === undefined) refuse('CONFLICT', "Docker was not read: the device's SSH user finds no docker command. Anima reads Docker only.");
  return found;
}

function detailOf(found: Row): Row {
  const running = found.state === 'RUNNING';
  return {
    id: found.id,
    name: found.name,
    image: found.image,
    imageId: `sha256:${identifier(90)}`,
    command: ['/entrypoint.sh', 'serve'],
    workingDir: '/app',
    user: '',
    restartPolicy: 'unless-stopped',
    state: found.state,
    exitCode: running ? 0 : 1,
    startedAt: '2026-11-18T08:00:00Z',
    finishedAt: running ? null : '2026-11-18T09:00:00Z',
    restartCount: 0,
    healthCheck: found.health === 'NONE' ? null : { test: ['CMD-SHELL', 'curl -f localhost:3000/health'], intervalSeconds: 30 },
    health: found.health,
    failingStreak: 0,
    lastHealthRun: found.health === 'NONE' ? null : { endedAt: '2026-11-18T10:59:30Z', exitCode: 0 },
    mounts: [{ type: 'volume', name: 'tracks-data', source: '/var/lib/docker/volumes/tracks-data/_data', destination: '/data', readOnly: false }],
    networks: [{ network: 'tracks_default', ipv4: '172.18.0.3', ipv6: null, gateway: '172.18.0.1', macAddress: '02:42:ac:12:00:03' }],
    labels: found.composeProject === null ? [] : [{ name: 'com.docker.compose.project', value: found.composeProject }],
    environmentNames: ['DATABASE_URL', 'SECRET_KEY_BASE', 'PATH'],
    processes: running ? [{ pid: 1, user: 'root', command: '/entrypoint.sh serve' }] : [],
  };
}

const STORAGE: Row = {
  images: [
    { id: `sha256:${identifier(90)}`, repository: 'example/tracks', tag: '0.36', size: '470MB', sharedSize: '120MB', uniqueSize: '350MB', created: '2026-11-16 12:24:00 +0000 UTC', containerCount: 2, containers: ['tracks-app-1', 'tracks-worker-1'] },
    { id: `sha256:${identifier(91)}`, repository: 'postgres', tag: '17', size: '438MB', sharedSize: '0B', uniqueSize: '438MB', created: '2026-11-05 10:00:31 +0000 UTC', containerCount: 1, containers: ['tracks-db-1'] },
    { id: `sha256:${identifier(92)}`, repository: null, tag: null, size: '13.6MB', sharedSize: '0B', uniqueSize: '13.6MB', created: '2026-10-01 09:00:00 +0000 UTC', containerCount: 0, containers: [] },
  ],
  volumes: [
    { name: 'tracks-data', driver: 'local', size: '232.9MB', containerCount: 1, containers: ['tracks-app-1'] },
    { name: 'old-data', driver: 'local', size: '1.2GB', containerCount: 0, containers: [] },
  ],
  usage: [
    { type: 'Images', totalCount: 3, activeCount: 2, size: '921.6MB', reclaimable: '13.6MB (1%)' },
    { type: 'Local Volumes', totalCount: 2, activeCount: 1, size: '1.433GB', reclaimable: '1.2GB (83%)' },
  ],
};

const UPDATE_STATES: Readonly<Record<string, Row>> = {
  'tracks-app-1': { state: 'NEWER_IMAGE', reason: null },
  'tracks-db-1': { state: 'UP_TO_DATE', reason: null },
  recorder: { state: 'NOT_KNOWN', reason: 'NO_LOCAL_DIGEST' },
};

export function containerFields(state: HubState): Record<string, unknown> {
  return {
    containers: ({ deviceId }: { deviceId: string }): readonly Row[] => containersOf(state, deviceId),
    containerDetail: ({ deviceId, containerId }: { deviceId: string; containerId: string }): Row => {
      const found = containersOf(state, deviceId).find((each) => String(each.id).startsWith(containerId));
      if (found === undefined) refuse('NOT_FOUND', 'The device has no container with this identifier.');
      return detailOf(found);
    },
    containerStorage: ({ deviceId }: { deviceId: string }): Row => {
      containersOf(state, deviceId);
      return STORAGE;
    },
    containerNetworks: ({ deviceId }: { deviceId: string }): readonly Row[] => [
      { id: identifier(80), name: 'tracks_default', driver: 'bridge', scope: 'local', internal: false, containers: containersOf(state, deviceId).filter((each) => each.composeProject !== null).map((each) => each.name) },
      { id: identifier(81), name: 'none', driver: 'null', scope: 'local', internal: false, containers: [] },
    ],
    containerImageUpdates: ({ deviceId }: { deviceId: string }): readonly Row[] =>
      containersOf(state, deviceId).map((each) => ({ containerId: each.id, container: each.name, image: each.image, state: 'UP_TO_DATE', reason: null, localDigest: null, registryDigest: null, ...UPDATE_STATES[String(each.name)] })),
    containerLog: ({ deviceId, containerId, lines }: { deviceId: string; containerId: string; lines?: number | null }): Row => {
      const found = containersOf(state, deviceId).find((each) => String(each.id).startsWith(containerId));
      if (found === undefined) refuse('NOT_FOUND', 'The device has no container with this identifier.');
      const count = Math.min(lines ?? 200, 12);
      const start = state.now().getTime() - count * 1000;
      return {
        lines: Array.from({ length: count }, (_, index) => ({ at: new Date(start + index * 1000).toISOString(), text: `${index % 4 === 3 ? 'WARN' : 'INFO'} ${String(found.name)}: request ${String(index + 1)} served` })),
        cut: false,
      };
    },
  };
}
