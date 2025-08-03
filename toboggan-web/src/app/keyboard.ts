/**
 * Keyboard Module
 * Handles keyboard shortcuts for presentation navigation
 */

import { KEYBOARD_SHORTCUTS } from "../utils/constants";
import type { CommandHandler } from "../types";


export class KeyboardModule {
  private readonly handler: CommandHandler;
  private isActive = false;

  constructor(handler: CommandHandler) {
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
