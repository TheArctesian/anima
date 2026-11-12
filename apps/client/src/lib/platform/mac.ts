const MAC = /Macintosh|Mac OS X/;
/** iPhone and iPod user agents say "like Mac OS X" too; they are not Macs. */
const IOS = /iPhone|iPod|iPad/;

/** Whether `userAgent` is a Mac's. Mac-only controls and texts ask this, never the user agent itself. */
export function isMac(userAgent: string): boolean {
  return MAC.test(userAgent) && !IOS.test(userAgent);
}
