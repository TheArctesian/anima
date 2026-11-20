/** What tells a phone or a tablet from a computer: the user agent and the number of touch points (`navigator`). */
export interface Agent {
  readonly userAgent: string;
  readonly maxTouchPoints: number;
}

const MOBILE = /iPhone|iPod|iPad|Android/;
/** An iPad can ask for the desktop site: its user agent then names a Mac, and only its touch points differ. */
const MAC = /Macintosh/;

/** Whether `agent` is an Android phone or tablet; its web view reports no inset for the gesture bar (MOB-011). */
export function isAndroid(agent: Agent): boolean {
  return /Android/.test(agent.userAgent);
}

export function isIPad(agent: Agent): boolean {
  return /iPad/.test(agent.userAgent) || (MAC.test(agent.userAgent) && agent.maxTouchPoints > 1);
}

/** Whether `agent` is an iPhone or an iPad: its app signs in inside the app (mobile plan M3-5). */
export function isApple(agent: Agent): boolean {
  return isMobile(agent) && !isAndroid(agent);
}

/** Whether `agent` is a phone or a tablet (MOB-004). Controls for a mobile device ask this, never the user agent itself. */
export function isMobile(agent: Agent): boolean {
  return MOBILE.test(agent.userAgent) || isIPad(agent);
}
