// What a subscription receives while the test runs: the fixtures hold no live events, so a spec pushes one by hand, as the
// hub would when a check changes a device's status. Subscribers are the open graphql-ws subscriptions, by operation name.

export interface Subscriber {
  readonly operation: string | null;
  /** Sends one `next` frame to the subscription. */
  deliver(data: Record<string, unknown>): void;
}

export interface Live {
  join(subscriber: Subscriber): () => void;
  /** Sends `data` to every open subscription of `operation`; returns how many got it. */
  push(operation: string, data: Record<string, unknown>): number;
}

export function newLive(): Live {
  const subscribers = new Set<Subscriber>();
  return {
    join(subscriber) {
      subscribers.add(subscriber);
      return () => subscribers.delete(subscriber);
    },
    push(operation, data) {
      const matching = [...subscribers].filter((found) => found.operation === operation);
      for (const subscriber of matching) subscriber.deliver(data);
      return matching.length;
    },
  };
}
