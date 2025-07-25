/**
 * Duration formatting utilities
 * Handles time display formatting using Intl.DateTimeFormat
 */

/**
 * Format duration in seconds to HH:MM:SS format
 * Uses Intl.DateTimeFormat for proper internationalization
 */
export function formatDuration(totalSeconds: number): string {
  // Create a date object at epoch + duration
  // We use UTC to avoid timezone issues
  const date = new Date(totalSeconds * 1000);
  
  // Use Intl.DateTimeFormat with user's locale
  // Using undefined as locale will use the browser's default locale
  const formatter = new Intl.DateTimeFormat(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23', // Use 24-hour format (00-23)
    timeZone: 'UTC'
  });
  
  return formatter.format(date);
}

/**
 * Calculate elapsed seconds from a start time
 */
export function calculateElapsedSeconds(startTime: Date): number {
  const now = new Date();
  const elapsedMs = now.getTime() - startTime.getTime();
  return Math.floor(elapsedMs / 1000);
}

/**
 * Calculate start time from a 'since' timestamp and total duration
 */
export function calculateStartTime(since: string, durationSecs: number, durationNanos: number): Date {
  const sinceDate = new Date(since);
  const totalDurationMs = durationSecs * 1000 + durationNanos / 1_000_000;
  return new Date(sinceDate.getTime() - totalDurationMs);
}