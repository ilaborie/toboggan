/**
 * Toboggan Web Presentation Application
 * Main entry point that initializes and coordinates all modules
 */

import type { AppConfig, Command } from './types.js';
import { PresentationController, type PresentationElements } from './controllers/presentationController.js';
import { getRequiredElement } from './utils/dom.js';

class TobogganApp {
  private readonly controller: PresentationController;
  private readonly navigationButtons: Map<string, Command>;

  constructor(config: AppConfig) {
    // Generate unique client ID
    const clientId = crypto.randomUUID();
    
    // Get all required DOM elements
    const elements = this.initializeElements();
    
    // Initialize presentation controller
    this.controller = new PresentationController(clientId, config, elements);
    
    // Set up navigation commands
    this.navigationButtons = new Map([
      ['first-btn', { command: 'First' } as Command],
      ['prev-btn', { command: 'Previous' } as Command],
      ['next-btn', { command: 'Next' } as Command],
      ['last-btn', { command: 'Last' } as Command],
      ['pause-btn', { command: 'Pause' } as Command],
      ['resume-btn', { command: 'Resume' } as Command],
    ]);
    
    // Attach event listeners
    this.attachEventListeners();
    
    // Start the application
    this.controller.start();
  }

  /**
   * Initialize and return all required DOM elements
   */
  private initializeElements(): PresentationElements {
    return {
      connectionStatus: getRequiredElement<HTMLSpanElement>('connection-status'),
      slideCounter: getRequiredElement<HTMLSpanElement>('slide-counter'),
      durationDisplay: getRequiredElement<HTMLSpanElement>('duration-display'),
      errorDisplay: getRequiredElement<HTMLDivElement>('error-display'),
      appElement: getRequiredElement<HTMLDivElement>('app')
    };
  }

  /**
   * Attach event listeners for navigation controls
   */
  private attachEventListeners(): void {
    // Navigation button clicks
    this.navigationButtons.forEach((command, buttonId) => {
      const button = document.getElementById(buttonId);
      if (button) {
        button.addEventListener('click', () => this.controller.sendCommand(command));
      }
    });

    // Keyboard navigation
    document.addEventListener('keydown', (e: KeyboardEvent) => this.handleKeydown(e));
  }

  /**
   * Handle keyboard shortcuts for navigation
   */
  private handleKeydown(event: KeyboardEvent): void {
    const keyCommands: Record<string, Command | undefined> = {
      'ArrowLeft': { command: 'Previous' },
      'ArrowUp': { command: 'Previous' },
      'ArrowRight': { command: 'Next' },
      'ArrowDown': { command: 'Next' },
      ' ': { command: 'Next' }, // Spacebar
      'Home': { command: 'First' },
      'End': { command: 'Last' },
      'p': { command: 'Pause' },
      'P': { command: 'Pause' },
      'r': { command: 'Resume' },
      'R': { command: 'Resume' }
    };

    const command = keyCommands[event.key];
    if (command) {
      event.preventDefault();
      this.controller.sendCommand(command);
    }
  }

  /**
   * Dispose of the application resources
   */
  public dispose(): void {
    this.controller.dispose();
  }
}

// Application configuration
const config: AppConfig = {
  wsUrl: 'ws://localhost:8080/api/ws',
  apiBaseUrl: 'http://localhost:8080',
  maxRetries: 5,
  initialRetryDelay: 1000,
  maxRetryDelay: 30000
};

// Initialize the application when the DOM is loaded
let app: TobogganApp | null = null;

document.addEventListener('DOMContentLoaded', (): void => {
  try {
    app = new TobogganApp(config);
    
    // Store reference globally for debugging (optional)
    if (typeof window !== 'undefined') {
      (window as any).tobogganApp = app;
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown initialization error';
    console.error('Failed to initialize Toboggan app:', errorMessage);
    
    // Show error in the UI if possible
    const errorDisplay = document.getElementById('error-display');
    if (errorDisplay) {
      errorDisplay.textContent = `Initialization failed: ${errorMessage}`;
      errorDisplay.style.display = 'block';
    }
  }
});

// Clean up on page unload
window.addEventListener('beforeunload', () => {
  if (app) {
    app.dispose();
  }
});