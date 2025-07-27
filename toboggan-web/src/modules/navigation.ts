/**
 * Navigation Module
 * Handles navigation button interactions
 */

import { ButtonId, NAVIGATION_BUTTONS } from "../constants/index.js";
import type { Command } from "../types.js";
import { TobogganNavigation } from "../components/navigation.js";

export interface NavigationHandler {
  onCommand: (command: Command) => void;
}

export class NavigationModule {
  private readonly handler: NavigationHandler;
  private readonly buttonListeners = new Map<string, () => void>();
  private navigationCommandListener: ((event: Event) => void) | undefined;

  constructor(handler: NavigationHandler) {
    this.handler = handler;
  }

  /**
   * Initialize navigation button event listeners
   */
  public initialize(): void {
    // Check if we have the navigation web component
    const navigationElement = document.getElementById("navigation") as TobogganNavigation;
    
    if (navigationElement && navigationElement.tagName === "TOBOGGAN-NAVIGATION") {
      // New web component approach - listen for custom events
      this.navigationCommandListener = (event: Event) => {
        const customEvent = event as CustomEvent;
        if (customEvent.detail && customEvent.detail.command) {
          this.handler.onCommand(customEvent.detail.command);
        }
      };
      
      navigationElement.addEventListener("navigation-command", this.navigationCommandListener);
    } else {
      // Legacy approach - set up individual button listeners
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
    const navigationElement = document.getElementById("navigation") as TobogganNavigation;
    
    if (navigationElement && navigationElement.tagName === "TOBOGGAN-NAVIGATION") {
      // For web component, check if the button exists within the component
      const command = NAVIGATION_BUTTONS[buttonId];
      return navigationElement.getButton(command.command) !== null;
    } else {
      // Legacy approach
      return document.getElementById(buttonId) !== null;
    }
  }

  /**
   * Enable or disable navigation buttons
   */
  public setButtonEnabled(buttonId: ButtonId, enabled: boolean): void {
    const navigationElement = document.getElementById("navigation") as TobogganNavigation;
    
    if (navigationElement && navigationElement.tagName === "TOBOGGAN-NAVIGATION") {
      const command = NAVIGATION_BUTTONS[buttonId];
      navigationElement.setButtonEnabled(command.command, enabled);
    } else {
      // Legacy approach
      const button = document.getElementById(buttonId) as HTMLButtonElement;
      if (button) {
        button.disabled = !enabled;
      }
    }
  }

  /**
   * Enable or disable all navigation buttons
   */
  public setAllButtonsEnabled(enabled: boolean): void {
    const navigationElement = document.getElementById("navigation") as TobogganNavigation;
    
    if (navigationElement && navigationElement.tagName === "TOBOGGAN-NAVIGATION") {
      navigationElement.setAllButtonsEnabled(enabled);
    } else {
      // Legacy approach
      Object.values(ButtonId).forEach((buttonId) => {
        this.setButtonEnabled(buttonId, enabled);
      });
    }
  }

  /**
   * Dispose of the module
   */
  public dispose(): void {
    const navigationElement = document.getElementById("navigation") as TobogganNavigation;
    
    if (navigationElement && this.navigationCommandListener) {
      // Remove web component event listener
      navigationElement.removeEventListener("navigation-command", this.navigationCommandListener);
      this.navigationCommandListener = undefined;
    } else {
      // Remove legacy button listeners
      this.buttonListeners.forEach((listener, buttonId) => {
        const button = document.getElementById(buttonId);
        if (button) {
          button.removeEventListener("click", listener);
        }
      });
    }
    
    this.buttonListeners.clear();
  }
}
