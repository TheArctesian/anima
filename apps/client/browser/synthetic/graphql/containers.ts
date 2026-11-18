import { GraphQLError } from 'graphql';
import type { Row } from '../fixtures';
import type { HubState } from '../model';

// The containers of a device, so the Containers tab has something to show (HOST-001, HOST-013). `nas` runs one Compose
// project and one container with none; `gpu-box` runs one container that is not healthy. Every other device answers as a
// device with no docker command does. Every image and name is an example one (CON-003).

const ID_DIGITS = '0123456789abcdef';

/** A 64-digit identifier made from a seed, so each container has the same one at every read. */
function identifier(seed: number): string {
  return Array.from({ length: 64 }, (_, index) => ID_DIGITS[(seed * 7 + index * 13) % 16]).join('');
}

function container(seed: number, change: Row): Row {
  return { id: identifier(seed), image: 'example/app:1', state: 'RUNNING', status: 'Up 3 hours', health: 'NONE', ports: [], createdAt: '2026-11-18T08:00:00Z', composeProject: null, composeService: null, ...change };
}

const BY_DEVICE: Readonly<Record<string, readonly Row[]>> = {
  nas: [
    container(1, { name: 'tracks-app-1', image: 'example/tracks:0.36', status: 'Up 3 hours (healthy)', health: 'HEALTHY', ports: ['0.0.0.0:3000->3000/tcp'], composeProject: 'tracks', composeService: 'app' }),
    container(2, { name: 'tracks-db-1', image: 'postgres:17', status: 'Up 3 hours (healthy)', health: 'HEALTHY', ports: ['5432/tcp'], composeProject: 'tracks', composeService: 'db' }),
    container(3, { name: 'tracks-worker-1', image: 'example/tracks:0.36', state: 'EXITED', status: 'Exited (1) 2 hours ago', composeProject: 'tracks', composeService: 'worker' }),
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

export function containerFields(state: HubState): Record<string, unknown> {
  return {
    containers: ({ deviceId }: { deviceId: string }): readonly Row[] => containersOf(state, deviceId),
    containerLog: ({ deviceId, containerId, lines }: { deviceId: string; containerId: string; lines?: number | null }): Row => {
      const found = containersOf(state, deviceId).find((each) => String(each.id).startsWith(containerId));
      if (found === undefined) refuse('NOT_FOUND', 'The device has no container with this identifier.');
      const count = Math.min(lines ?? 200, 12);
      const start = state.now().getTime() - count * 1000;
      return {
        lines: Array.from({ length: count }, (_, index) => ({ at: new Date(start + index * 1000).toISOString(), text: `${String(found.name)}: request ${String(index + 1)} served` })),
        cut: false,
      };
    },
  };
}
