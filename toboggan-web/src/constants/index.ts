/**
 * Application Constants
 * Centralized constants for better maintainability
 */

import type { Command } from "../types.js";

/**
 * Button IDs enum for type safety and maintainability
 */
export enum ButtonId {
  FIRST = "first-btn",
  PREVIOUS = "prev-btn",
  NEXT = "next-btn",
  LAST = "last-btn",
  PAUSE = "pause-btn",
  RESUME = "resume-btn",
}

/**
 * Element IDs for DOM access
 */
export enum ElementId {
  CONNECTION_STATUS = "connection-status",
  SLIDE_COUNTER = "slide-counter",
  DURATION_DISPLAY = "duration-display",
  ERROR_DISPLAY = "error-display",
  APP = "app",
  TOAST_CONTAINER = "toast-container",
}

/**
 * Command constants for better performance and consistency
 */
export const COMMANDS = {
  FIRST: { command: "First" } as const satisfies Command,
  PREVIOUS: { command: "Previous" } as const satisfies Command,
  NEXT: { command: "Next" } as const satisfies Command,
  LAST: { command: "Last" } as const satisfies Command,
  PAUSE: { command: "Pause" } as const satisfies Command,
  RESUME: { command: "Resume" } as const satisfies Command,
  PING: { command: "Ping" } as const satisfies Command,
} as const;

/**
 * Keyboard shortcuts mapping
 */
export const KEYBOARD_SHORTCUTS: Record<string, Command> = {
  ArrowLeft: COMMANDS.PREVIOUS,
  ArrowUp: COMMANDS.PREVIOUS,
  ArrowRight: COMMANDS.NEXT,
  ArrowDown: COMMANDS.NEXT,
  " ": COMMANDS.NEXT, // Spacebar
  Home: COMMANDS.FIRST,
  End: COMMANDS.LAST,
  p: COMMANDS.PAUSE,
  P: COMMANDS.PAUSE,
  r: COMMANDS.RESUME,
  R: COMMANDS.RESUME,
} as const;

/**
 * Navigation button to command mapping
 */
export const NAVIGATION_BUTTONS: Record<ButtonId, Command> = {
  [ButtonId.FIRST]: COMMANDS.FIRST,
  [ButtonId.PREVIOUS]: COMMANDS.PREVIOUS,
  [ButtonId.NEXT]: COMMANDS.NEXT,
  [ButtonId.LAST]: COMMANDS.LAST,
  [ButtonId.PAUSE]: COMMANDS.PAUSE,
  [ButtonId.RESUME]: COMMANDS.RESUME,
} as const;

/**
 * Default configuration values
 */
export const DEFAULTS = {
  PING_INTERVAL: 30000, // 30 seconds
  ERROR_DISPLAY_DURATION: 5000, // 5 seconds
  TOAST_DURATION: 3000, // 3 seconds
} as const;
