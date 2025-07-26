/**
 * Navigation Module
 * Handles navigation button interactions
 */

import { ButtonId, NAVIGATION_BUTTONS } from "../constants/index.js";
import type { Command } from "../types.js";

export interface NavigationHandler {
  onCommand: (command: Command) => void;
}

export class NavigationModule {
  private readonly handler: NavigationHandler;
  private readonly buttonListeners = new Map<string, () => void>();

  constructor(handler: NavigationHandler) {
    this.handler = handler;
  }

  /**
   * Initialize navigation button event listeners
   */
  public initialize(): void {
    // Set up navigation button clicks
    Object.values(ButtonId).forEach((buttonId) => {
      const button = document.getElementById(buttonId);
      if (button) {
        const command = NAVIGATION_BUTTONS[buttonId];
        const listener = () => this.handler.onCommand(command);

        button.addEventListener("click", listener);
        this.buttonListeners.set(buttonId, listener);
      }
    });
  }

  /**
   * Get the command associated with a button ID
   */
  public getCommand(buttonId: ButtonId): Command | undefined {
    return NAVIGATION_BUTTONS[buttonId];
  }

  /**
   * Check if a button exists in the DOM
   */
  public hasButton(buttonId: ButtonId): boolean {
    return document.getElementById(buttonId) !== null;
  }

  /**
   * Dispose of the module
   */
  public dispose(): void {
    // Remove all event listeners
    this.buttonListeners.forEach((listener, buttonId) => {
      const button = document.getElementById(buttonId);
      if (button) {
        button.removeEventListener("click", listener);
      }
    });
    this.buttonListeners.clear();
  }
}
