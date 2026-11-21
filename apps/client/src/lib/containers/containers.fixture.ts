import type { HubError, HubResult } from '$lib/hub/hub';
import type { CatalogueEntry } from '$lib/catalogue/api';
import type { Container, ContainerCalls, ContainerDetail, ContainerLog, ContainerNetwork, ContainerStorage, ImageUpdate } from './api';

// Test support: containers in the hub's shape, and calls that answer with them and record what the screen asked.

export const APP_ID = `4f5c1d2e3a4b${'0'.repeat(52)}`;

export function container(change: Partial<Container>): Container {
  return { id: APP_ID, name: 'tracks-app-1', image: 'example/tracks:0.36', state: 'RUNNING', status: 'Up 3 hours (healthy)', health: 'HEALTHY', ports: ['0.0.0.0:3000->3000/tcp'], createdAt: '2026-11-18T08:00:00Z', composeProject: 'tracks', composeService: 'app', composeProjectState: 'PARTLY_RUNNING', ...change };
}

export const CONTAINERS: readonly Container[] = [
  container({ id: 'c'.repeat(64), name: 'recorder', image: 'example/recorder:1.0', status: 'Up 9 days', health: 'NONE', ports: [], composeProject: null, composeService: null, composeProjectState: null }),
  container({ id: 'b'.repeat(64), name: 'tracks-worker-1', state: 'EXITED', status: 'Exited (1) 2 hours ago', health: 'NONE', ports: [], composeService: 'worker' }),
  container({}),
  container({ id: 'a'.repeat(64), name: 'ollama', image: 'example/ollama:0.9', status: 'Up 2 minutes (unhealthy)', health: 'UNHEALTHY', ports: [], composeProject: 'ai', composeService: 'ollama', composeProjectState: 'UNHEALTHY' }),
];

export const LOG: ContainerLog = {
  lines: [
    { at: '2026-11-18T18:00:00+00:00', text: 'listening on :3000' },
    { at: null, text: '<b>ready</b>' },
  ],
  cut: false,
};

/** The details of `tracks-app-1`: a health check, a volume and a bind mount, one network. */
export const DETAIL: ContainerDetail = {
  id: APP_ID,
  name: 'tracks-app-1',
  image: 'example/tracks:0.36',
  imageId: `sha256:${'5'.repeat(64)}`,
  command: ['web-entrypoint.sh', 'bin/rails', 'server'],
  workingDir: '/var/app',
  user: '',
  restartPolicy: 'on-failure:3',
  state: 'RUNNING',
  exitCode: 0,
  startedAt: '2026-11-18T08:00:00+00:00',
  finishedAt: null,
  restartCount: 2,
  healthCheck: { test: ['CMD-SHELL', 'curl -f localhost:3000'], intervalSeconds: 10 },
  health: 'HEALTHY',
  failingStreak: 0,
  lastHealthRun: { endedAt: '2026-11-18T18:00:00+00:00', exitCode: 0 },
  mounts: [
    { type: 'volume', name: 'tracks-data', source: '/var/lib/docker/volumes/tracks-data/_data', destination: '/data', readOnly: false },
    { type: 'bind', name: null, source: '/srv/config', destination: '/config', readOnly: true },
  ],
  networks: [{ network: 'tracks_default', ipv4: '172.18.0.3', ipv6: null, gateway: '172.18.0.1', macAddress: '6e:c5:bc:92:d3:2a' }],
  labels: [{ name: 'com.docker.compose.project', value: '<i>tracks</i>' }],
  environmentNames: ['DATABASE_PASSWORD', 'RAILS_ENV'],
  processes: [{ pid: 805, user: 'root', command: 'bin/rails server' }],
};

export const STORAGE: ContainerStorage = {
  images: [
    { id: `sha256:${'5'.repeat(64)}`, repository: 'example/tracks', tag: '0.36', size: '470MB', sharedSize: '420.8MB', uniqueSize: '49.49MB', created: '2026-11-16 12:24:00 -0800 PST', containerCount: 3, containers: ['tracks-app-1'] },
    { id: `sha256:f59abe656b69${'0'.repeat(52)}`, repository: null, tag: null, size: '13.6MB', sharedSize: '0B', uniqueSize: '13.6MB', created: '2026-11-05 10:00:31 -0800 PST', containerCount: 0, containers: [] },
  ],
  volumes: [
    { name: 'tracks-data', driver: 'local', size: '232.9MB', containerCount: 1, containers: ['tracks-app-1'] },
    { name: 'old-data', driver: 'local', size: '1.2GB', containerCount: 0, containers: [] },
  ],
  usage: [
    { type: 'Images', totalCount: 2, activeCount: 1, size: '483.6MB', reclaimable: '13.6MB (2%)' },
    { type: 'Local Volumes', totalCount: 2, activeCount: 1, size: '1.433GB', reclaimable: '1.2GB (83%)' },
  ],
};

export const NETWORKS: readonly ContainerNetwork[] = [
  { id: 'a1'.repeat(32), name: 'tracks_default', driver: 'bridge', scope: 'local', internal: false, containers: ['tracks-app-1', 'tracks-worker-1'] },
  { id: 'b2'.repeat(32), name: 'backend', driver: 'bridge', scope: 'local', internal: true, containers: [] },
];

function update(containerId: string, container: string, change: Partial<ImageUpdate>): ImageUpdate {
  return { containerId, container, image: 'example/tracks:0.36', state: 'UP_TO_DATE', reason: null, localDigest: null, registryDigest: null, ...change };
}

export const UPDATES: readonly ImageUpdate[] = [
  update(APP_ID, 'tracks-app-1', { state: 'NEWER_IMAGE' }),
  update('b'.repeat(64), 'tracks-worker-1', {}),
  update('c'.repeat(64), 'recorder', { state: 'NOT_KNOWN', reason: 'NO_LOCAL_DIGEST' }),
];

const GENERATED = {
  tier: 'GENERATED',
  origin: { publisher: 'LinuxServer.io', read: '2026-11-17' },
  install: { refusal: 'GENERATED_NEEDS_DIGEST', reason: 'Anima did not test this entry: it is generated from the data of its publisher. To install it, a person confirms the digest of its image.', needsOpenToNetwork: false },
} as const;

/** Tasks H38 and H39: two tested entries and two generated ones (a shell, and one with a stream and an optional device). */
export const ENTRIES: readonly CatalogueEntry[] = [
  entry('owntracks-recorder', 'OwnTracks Recorder', { lastMachineTest: '2026-11-20', tags: ['location'] }),
  entry('postgresql', 'PostgreSQL', { kind: 'DATABASE', tags: ['database'] }),
  entry('code-server', 'code-server', { ...GENERATED, class: 'SHELL', tags: ['linuxserver', 'programming'] }),
  entry('jellyfin', 'jellyfin', {
    ...GENERATED,
    tags: ['linuxserver', 'media servers'],
    streams: [{ port: 7359, protocols: ['tcp', 'udp'] }],
    optionalParts: [{ kind: 'DEVICE', path: '/dev/dri', on: false, needsMachineSetting: true }],
  }),
];

function entry(id: string, name: string, changes: Partial<CatalogueEntry>): CatalogueEntry {
  return {
    id,
    name,
    description: `${name} for the tests.`,
    kind: 'APPLICATION',
    version: '1.0.0',
    author: 'Example',
    tags: ['test'],
    documentationUrl: 'https://example.org/docs',
    marks: [],
    networkPorts: [],
    class: 'APPLICATION',
    tier: 'TESTED',
    origin: null,
    streams: [],
    optionalParts: [],
    lastMachineTest: null,
    pinned: '2026-11-20',
    install: { refusal: 'INSTALL_NOT_BUILT', reason: 'The install needs task H6 of the hosted applications plan, which is not built. No machine was reached.', needsOpenToNetwork: false },
    environmentVariables: ['SECRET_KEY', 'PORT'],
    composeFile: 'services:\n  app:\n    image: example/app@sha256:0\n',
    ...changes,
  };
}

export interface FakeCalls extends ContainerCalls {
  /** Each read of details, in order: the container. */
  readonly detailsAsked: string[];
  storageAsked: number;
  networksAsked: number;
  updatesAsked: number;
  entriesAsked: number;
  /** Each log read, in order: the container and the number of lines. */
  readonly logsAsked: [string, number][];
  listsAsked: number;
  /** What the next call answers instead of its answer. */
  fail: HubError | null;
  containers: readonly Container[];
  logAnswer: ContainerLog;
}

export function fakeCalls(containers: readonly Container[] = CONTAINERS): FakeCalls {
  const answer = <T>(value: T): Promise<HubResult<T>> => Promise.resolve(calls.fail === null ? { ok: true, value } : { ok: false, error: calls.fail });
  const calls: FakeCalls = {
    logsAsked: [],
    detailsAsked: [],
    storageAsked: 0,
    networksAsked: 0,
    updatesAsked: 0,
    entriesAsked: 0,
    listsAsked: 0,
    fail: null,
    containers,
    logAnswer: LOG,
    list: () => {
      calls.listsAsked += 1;
      return answer(calls.containers);
    },
    log: (containerId, lines) => {
      calls.logsAsked.push([containerId, lines]);
      return answer(calls.logAnswer);
    },
    detail: (containerId) => {
      calls.detailsAsked.push(containerId);
      return answer(DETAIL);
    },
    storage: () => {
      calls.storageAsked += 1;
      return answer(STORAGE);
    },
    networks: () => {
      calls.networksAsked += 1;
      return answer(NETWORKS);
    },
    updates: () => {
      calls.updatesAsked += 1;
      return answer(UPDATES);
    },
    entries: () => {
      calls.entriesAsked += 1;
      return answer(ENTRIES);
    },
  };
  return calls;
}
