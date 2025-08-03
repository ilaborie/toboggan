/**
 * Application Constants
 * Centralized constants for better maintainability
 */

import type { Command } from "../types";

/**
 * Command constants for better performance and consistency
 */
export const COMMANDS: Record<string, Command> = {
  FIRST: { command: "First" },
  PREVIOUS: { command: "Previous" },
  NEXT: { command: "Next" },
  LAST: { command: "Last" },
  PAUSE: { command: "Pause" },
  RESUME: { command: "Resume" },
  PING: { command: "Ping" },
  BLINK: { command: "Blink" },
} as const;

/**
 * Keyboard shortcuts mapping
 */
export const KEYBOARD_SHORTCUTS: Record<string, Command> = {
  ArrowLeft: COMMANDS.PREVIOUS,
  ArrowUp: COMMANDS.PREVIOUS,
  ArrowRight: COMMANDS.NEXT,
  ArrowDown: COMMANDS.NEXT,
  " ": COMMANDS.NEXT, // Space bar
  Home: COMMANDS.FIRST,
  End: COMMANDS.LAST,
  p: COMMANDS.PAUSE,
  P: COMMANDS.PAUSE,
  r: COMMANDS.RESUME,
  R: COMMANDS.RESUME,
  b: COMMANDS.BLINK,
  B: COMMANDS.BLINK,
} as const;

/**
 * Default configuration values
 */
export const DEFAULTS = {
  PING_INTERVAL: 60_000, // 1m
} as const;
