/**
 * Timer Service
 * Manages presentation duration timing
 */

import type { State } from '../types.js';
import { formatDuration, calculateElapsedSeconds, calculateStartTime } from '../utils/duration.js';

export class TimerService {
  private interval: number | null = null;
  private startTime: Date | null = null;
  private readonly durationDisplay: HTMLElement;
  private currentState: State | null = null;

  constructor(durationDisplay: HTMLElement) {
    this.durationDisplay = durationDisplay;
  }

  /**
   * Update timer based on presentation state
   */
  public updateState(state: State): void {
    this.currentState = state;
    
    if ('Running' in state) {
      // Calculate start time from 'since' timestamp and total_duration
      this.startTime = calculateStartTime(
        state.Running.since,
        state.Running.total_duration.secs,
        state.Running.total_duration.nanos
      );
      this.start();
    } else if ('Paused' in state) {
      this.stop();
      // Display the paused duration
      this.updateDisplay(state.Paused.total_duration.secs);
    } else if ('Done' in state) {
      this.stop();
      // Display the final duration
      this.updateDisplay(state.Done.total_duration.secs);
    }
  }

  /**
   * Stop the timer
   */
  public stop(): void {
    if (this.interval !== null) {
      window.clearInterval(this.interval);
      this.interval = null;
    }
  }

  /**
   * Start the timer
   */
  private start(): void {
    // Clear any existing timer
    this.stop();
    
    // Update immediately
    this.updateFromStartTime();
    
    // Update every second
    this.interval = window.setInterval(() => {
      this.updateFromStartTime();
    }, 1000);
  }

  /**
   * Update display from start time
   */
  private updateFromStartTime(): void {
    if (!this.startTime || !this.currentState || !('Running' in this.currentState)) {
      return;
    }
    
    const elapsedSeconds = calculateElapsedSeconds(this.startTime);
    this.updateDisplay(elapsedSeconds);
  }

  /**
   * Update the duration display
   */
  private updateDisplay(totalSeconds: number): void {
    const formatted = formatDuration(totalSeconds);
    this.durationDisplay.textContent = `Duration: ${formatted}`;
  }
}