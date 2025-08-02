/**
 * Keyboard Module
 * Handles keyboard shortcuts for presentation navigation
 */

import { KEYBOARD_SHORTCUTS } from "../utils/constants";
import type { Command } from "../types";

export interface KeyboardHandler {
  onCommand: (command: Command) => void;
}

export class KeyboardModule {
  private readonly handler: KeyboardHandler;
  private isActive = false;

  constructor(handler: KeyboardHandler) {
    this.handler = handler;
  }

  /**
   * Start listening for keyboard events
   */
  public start(): void {
    if (this.isActive) return;

    document.addEventListener("keydown", this.handleKeydown);
    this.isActive = true;
  }

  /**
   * Stop listening for keyboard events
   */
  public stop(): void {
    if (!this.isActive) return;

    document.removeEventListener("keydown", this.handleKeydown);
    this.isActive = false;
  }

  /**
   * Handle keyboard shortcuts for navigation
   */
  private readonly handleKeydown = (event: KeyboardEvent): void => {
    const command = KEYBOARD_SHORTCUTS[event.key];
    if (command) {
      event.preventDefault();
      this.handler.onCommand(command);
    }
  };

  /**
   * Dispose of the module
   */
  public dispose(): void {
    this.stop();
  }
}
