import { graphql } from '$lib/hub/generated';
import type { EventsQuery, EventsQueryVariables, Severity } from '$lib/hub/generated/graphql';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

const Events = graphql(`
  query Events($filter: EventFilter, $first: Int, $after: String) {
    events(filter: $filter, first: $first, after: $after) {
      pageInfo {
        hasNextPage
        endCursor
      }
      edges {
        node {
          id
          at
          severity
          source
          deviceId
          deviceName
          message
        }
      }
    }
  }
`);

const EventAdded = graphql(`
  subscription EventAdded($minSeverity: Severity) {
    eventAdded(minSeverity: $minSeverity) {
      id
      at
      severity
      source
      deviceId
      deviceName
      message
    }
  }
`);

/** One page of the event log, as the hub answers it. */
export type EventPage = EventsQuery['events'];

/** One log row. */
export type LogEvent = EventPage['edges'][number]['node'];

/** The event log and live events through a `Hub` (CQ-045). */
export function eventsApi(hub: Hub) {
  return {
    list: async (variables: EventsQueryVariables): Promise<HubResult<EventPage>> => {
      const result = await hub.request(Events, variables);
      return result.ok ? { ok: true, value: result.value.events } : result;
    },
    /** Listen for events of `minSeverity` and above (LOG-010); returns the function that stops listening. */
    watch: (minSeverity: Severity, sink: Sink<LogEvent>): (() => void) =>
      hub.subscribe(
        EventAdded,
        { minSeverity },
        {
          next: (value) => {
            sink.next(value.eventAdded);
          },
          error: (error) => {
            sink.error(error);
          },
          complete: () => {
            sink.complete();
          },
        },
      ),
  };
}
