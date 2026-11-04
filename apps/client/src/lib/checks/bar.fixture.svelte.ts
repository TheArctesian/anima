import type { Beat } from './heartbeats';

/** The beats a mounted `HeartbeatBar` shows in a test; assign `beats` to add one. */
export const shown: { beats: readonly Beat[] } = $state({ beats: [] });
