import { graphql } from '$lib/hub/generated';
import type { LiveSessionChangedSubscription, LiveSessionsQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';
import { sessionsApi, type EndedSession } from '../api';

const LiveSessions = graphql(`
  query LiveSessions($first: Int) {
    sessions(live: true, first: $first) {
      edges {
        node {
          id
          deviceId
          deviceName
          protocol
          operatorName
          startedAt
          clientAddr
        }
      }
    }
  }
`);

const LiveSessionChanged = graphql(`
  subscription LiveSessionChanged {
    sessionChanged {
      session {
        id
        deviceId
        deviceName
        protocol
        operatorName
        startedAt
        clientAddr
        endedAt
        endReason
      }
    }
  }
`);

/** A session that is running now, with the computer that holds it (RA-034). */
export type LiveSession = LiveSessionsQuery['sessions']['edges'][number]['node'];

/** A session as `sessionChanged` sends it: ended when `endedAt` is set. */
export type SessionChange = LiveSessionChangedSubscription['sessionChanged']['session'];

/** The live-session calls through a `Hub` (RA-034). Failures stay the hub's typed K7 errors. */
export interface LiveSessionsApi {
  /** The running sessions, up to `first`. */
  list(first: number): Promise<HubResult<readonly LiveSession[]>>;
  /** Listen for starts and ends; returns the function that stops listening. Errors reach `sink.error`. */
  watch(sink: Sink<SessionChange>): () => void;
  /** End the session (D-4). */
  end(id: string): Promise<HubResult<EndedSession>>;
}

export function liveSessionsApi(hub: Hub): LiveSessionsApi {
  const sessions = sessionsApi(hub);
  return {
    list: async (first) => {
      const result = await hub.request(LiveSessions, { first });
      return result.ok ? { ok: true, value: result.value.sessions.edges.map((edge) => edge.node) } : result;
    },
    watch: (sink) =>
      hub.subscribe(
        LiveSessionChanged,
        {},
        {
          next: (value) => {
            sink.next(value.sessionChanged.session);
          },
          error: (error) => {
            sink.error(error);
          },
          complete: () => {
            sink.complete();
          },
        },
      ),
    end: (id) => sessions.endSession(id),
  };
}
