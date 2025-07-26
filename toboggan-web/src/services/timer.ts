/**
 * Timer Service
 * Manages presentation duration timing
 */

import type { State } from "../types.js";
import { calculateElapsedSeconds, calculateStartTime, formatDuration } from "../utils/duration.js";

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

    if (state.state === "Running") {
      // Calculate start time from 'since' timestamp and total_duration
      this.startTime = calculateStartTime(state.since, state.total_duration.secs);
      this.start();
    } else if (state.state === "Paused") {
      this.stop();
      // Display the paused duration
      this.updateDisplay(state.total_duration.secs);
    } else if (state.state === "Done") {
      this.stop();
      // Display the final duration
      this.updateDisplay(state.total_duration.secs);
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
    if (!this.startTime || !this.currentState || this.currentState?.state !== "Running") {
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
