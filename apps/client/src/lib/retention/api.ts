import { graphql } from '$lib/hub/generated';
import type { RetentionQuery, RetentionSubject } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

// Own documents (WP-50 F-3): the K1 retention fields are @pending, so a hub without them fails these documents only,
// and only the Data retention section shows an error.
const RETENTION = graphql(`
  query Retention {
    retention {
      subject
      days
      minDays
      maxDays
      defaultDays
    }
  }
`);

const SET_RETENTION = graphql(`
  mutation SetRetention($subject: RetentionSubject!, $days: Int!) {
    setRetention(subject: $subject, days: $days) {
      subject
      days
      minDays
      maxDays
      defaultDays
    }
  }
`);

/** How long the hub keeps one kind of data, with the bounds it accepts and its default (HUB-005, D-25). */
export type RetentionPeriod = RetentionQuery['retention'][number];

export type { RetentionSubject };

/** The retention calls of the hub (HUB-005) as plain functions of a `Hub`. */
export interface RetentionApi {
  list(): Promise<HubResult<readonly RetentionPeriod[]>>;
  /** The period as the hub now holds it. */
  set(subject: RetentionSubject, days: number): Promise<HubResult<RetentionPeriod>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function retentionApi(hub: Hub): RetentionApi {
  return {
    list: async () => mapValue(await hub.request(RETENTION, {}), (value) => value.retention),
    set: async (subject, days) => mapValue(await hub.request(SET_RETENTION, { subject, days }), (value) => value.setRetention),
  };
}
