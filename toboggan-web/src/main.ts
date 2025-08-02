/**
 * Toboggan Web Presentation Application
 * Main entry point that initializes and coordinates all modules
 */

import { appConfig } from "./config";
import { PresentationController, } from "./controllers/presentationController";
import { loadPresentationElements, PresentationElements } from "./elements";
import { type KeyboardHandler, KeyboardModule } from "./utils/keyboard";

import type { Command } from "./types";

import "./components/toast";
import "./components/navigation";
import "./components/toast";

class TobogganApp implements KeyboardHandler {
  private readonly elementsModule: PresentationElements;

  private readonly controller: PresentationController;
  private readonly keyboardModule: KeyboardModule;

  constructor() {
    const clientId = crypto.randomUUID();
    this.elementsModule = loadPresentationElements();
    this.keyboardModule = new KeyboardModule(this);
    this.controller = new PresentationController(clientId, appConfig, this.elementsModule);

    this.keyboardModule.start();
    this.controller.start();
  }

  /**
   * Handle commands from keyboard and navigation modules
   */
  public onCommand(command: Command): void {
    this.controller.sendCommand(command);
  }

  /**
   * Dispose of the application resources
   */
  public dispose(): void {
    this.keyboardModule.dispose();
    this.controller.dispose();
  }
}

// Initialize the application when the DOM is loaded
let app: TobogganApp | null = null;
document.addEventListener("DOMContentLoaded", (): void => {
  app = new TobogganApp();
});

// Clean up on page unload
window.addEventListener("beforeunload", () => {
  if (app) {
    app.dispose();
  }
});
