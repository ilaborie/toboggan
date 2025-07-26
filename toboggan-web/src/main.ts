/**
 * Toboggan Web Presentation Application
 * Main entry point that initializes and coordinates all modules
 */

import { ErrorComponent } from "./components/error.js";
import { appConfig } from "./config/index.js";
import { PresentationController } from "./controllers/presentationController.js";
import { ElementsModule } from "./modules/elements.js";
import { type KeyboardHandler, KeyboardModule } from "./modules/keyboard.js";
import { type NavigationHandler, NavigationModule } from "./modules/navigation.js";
import { ToastService } from "./services/toast.js";
import type { Command } from "./types.js";

// Import web components to ensure they are registered
import "./components/toast.js";
import "./components/error.js";
// Import service modules to register toast container component
import "./services/toast.js";

class TobogganApp implements KeyboardHandler, NavigationHandler {
  private readonly controller: PresentationController;
  private readonly keyboardModule: KeyboardModule;
  private readonly navigationModule: NavigationModule;
  private readonly elementsModule: ElementsModule;
  private readonly toastService: ToastService;
  private readonly errorComponent: ErrorComponent;

  constructor() {
    // Initialize modules
    this.elementsModule = new ElementsModule();
    this.keyboardModule = new KeyboardModule(this);
    this.navigationModule = new NavigationModule(this);

    // Validate DOM elements
    const validation = this.elementsModule.validate();
    if (!validation.valid) {
      throw new Error(`Missing required DOM elements: ${validation.missing.join(", ")}`);
    }

    // Initialize DOM elements
    const elements = this.elementsModule.initialize();

    // Initialize services
    this.toastService = new ToastService();
    this.errorComponent = new ErrorComponent(elements.errorDisplay);

    // Generate unique client ID
    const clientId = crypto.randomUUID();

    // Initialize presentation controller
    this.controller = new PresentationController(clientId, appConfig, elements);

    // Set up modules
    this.initializeModules();

    // Start the application
    this.controller.start();
  }

  /**
   * Handle commands from keyboard and navigation modules
   */
  public onCommand(command: Command): void {
    this.controller.sendCommand(command);
  }

  /**
   * Initialize all modules
   */
  private initializeModules(): void {
    this.navigationModule.initialize();
    this.keyboardModule.start();
  }

  /**
   * Get the toast service instance
   */
  public getToastService(): ToastService {
    return this.toastService;
  }

  /**
   * Get the error component instance
   */
  public getErrorComponent(): ErrorComponent {
    return this.errorComponent;
  }

  /**
   * Dispose of the application resources
   */
  public dispose(): void {
    this.keyboardModule.dispose();
    this.navigationModule.dispose();
    this.errorComponent.dispose();
    this.controller.dispose();
  }
}

// Initialize the application when the DOM is loaded
let app: TobogganApp | null = null;

document.addEventListener("DOMContentLoaded", (): void => {
  try {
    app = new TobogganApp();

    // Store reference globally for debugging (optional)
    if (typeof window !== "undefined") {
      (window as typeof window & { tobogganApp?: TobogganApp }).tobogganApp = app;
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : "Unknown initialization error";
    console.error("Failed to initialize Toboggan app:", errorMessage);

    // Show error in the UI if possible
    const errorDisplay = document.getElementById("error-display");
    if (errorDisplay) {
      errorDisplay.textContent = `Initialization failed: ${errorMessage}`;
      errorDisplay.style.display = "block";
    }
  }
});

// Clean up on page unload
window.addEventListener("beforeunload", () => {
  if (app) {
    app.dispose();
  }
});
