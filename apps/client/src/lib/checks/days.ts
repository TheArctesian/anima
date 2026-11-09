/** A count of days in words: "1 day" for one or minus one, "N days" otherwise. */
export function dayCount(days: number): string {
  return `${String(days)} ${Math.abs(days) === 1 ? 'day' : 'days'}`;
}
