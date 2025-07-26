/**
 * Error Display Web Component
 * Native web component that handles error display/hide status and console logging
 */

export interface ErrorDisplayConfig {
  autoHideDuration?: number;
  logToConsole?: boolean;
}

/**
 * Custom element for displaying error messages with shadow DOM encapsulation
 */
export class TobogganErrorDisplay extends HTMLElement {
  declare root: ShadowRoot;
  private errorContainer!: HTMLDivElement;
  private config: ErrorDisplayConfig;
  private hideTimeout: number | null = null;

  constructor() {
    super();

    // Create shadow DOM
    this.root = this.attachShadow({ mode: "open" });

    // Set default configuration
    this.config = {
      autoHideDuration: 5000,
      logToConsole: true,
    };

    // Create the internal structure
    this.createStructure();
    this.applyStyles();
  }

  /**
   * Observed attributes for the web component
   */
  static get observedAttributes(): string[] {
    return ["auto-hide-duration", "log-to-console"];
  }

  /**
   * Handle attribute changes
   */
  attributeChangedCallback(name: string, _oldValue: string | null, newValue: string | null): void {
    switch (name) {
      case "auto-hide-duration":
        this.config.autoHideDuration = newValue ? parseInt(newValue, 10) : 5000;
        break;
      case "log-to-console":
        this.config.logToConsole = newValue !== "false";
        break;
    }
  }

  /**
   * Show an error message
   */
  public show(message: string, error?: Error): void {
    // Log to console if enabled
    if (this.config.logToConsole) {
      if (error) {
        console.error(message, error);
      } else {
        console.error(message);
      }
    }

    // Clear any existing timeout
    this.clearHideTimeout();

    // Display error in UI
    this.errorContainer.textContent = message;
    this.errorContainer.style.display = "block";
    this.setAttribute("aria-hidden", "false");

    // Dispatch custom event for error shown
    this.dispatchEvent(
      new CustomEvent("error-shown", {
        detail: { message, error },
        bubbles: true,
      })
    );

    // Auto-hide if duration is specified
    if (this.config.autoHideDuration && this.config.autoHideDuration > 0) {
      this.hideTimeout = window.setTimeout(() => {
        this.hide();
      }, this.config.autoHideDuration);
    }
  }

  /**
   * Hide the error message
   */
  public hide(): void {
    this.clearHideTimeout();
    this.errorContainer.style.display = "none";
    this.setAttribute("aria-hidden", "true");

    // Dispatch custom event for error hidden
    this.dispatchEvent(new CustomEvent("error-hidden", { bubbles: true }));
  }

  /**
   * Check if error is currently visible
   */
  public get isVisible(): boolean {
    return this.errorContainer.style.display === "block";
  }

  /**
   * Clear any pending hide timeout
   */
  private clearHideTimeout(): void {
    if (this.hideTimeout !== null) {
      clearTimeout(this.hideTimeout);
      this.hideTimeout = null;
    }
  }

  /**
   * Create the internal DOM structure
   */
  private createStructure(): void {
    this.errorContainer = document.createElement("div");
    this.errorContainer.setAttribute("role", "alert");
    this.errorContainer.setAttribute("aria-live", "assertive");
    this.errorContainer.style.display = "none";

    this.root.appendChild(this.errorContainer);
  }

  /**
   * Apply styles to the shadow DOM
   */
  private applyStyles(): void {
    const style = document.createElement("style");
    style.textContent = `
      :host {
        display: block;
        width: 100%;
      }

      :host([hidden]) {
        display: none !important;
      }

      div {
        background-color: #fee;
        color: #c53030;
        border: 1px solid #feb2b2;
        border-radius: 4px;
        padding: 12px 16px;
        margin: 8px 0;
        font-family: system-ui, -apple-system, sans-serif;
        font-size: 14px;
        line-height: 1.4;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        word-wrap: break-word;
      }

      div:empty {
        display: none;
      }
    `;

    this.root.appendChild(style);
  }

  /**
   * Cleanup when element is removed from DOM
   */
  disconnectedCallback(): void {
    this.clearHideTimeout();
  }
}

/**
 * Legacy ErrorComponent class for backward compatibility
 * Wraps the web component to maintain the same API
 */
export class ErrorComponent {
  private readonly element: TobogganErrorDisplay;

  constructor(element: HTMLElement | TobogganErrorDisplay, config: ErrorDisplayConfig = {}) {
    if (element instanceof TobogganErrorDisplay) {
      this.element = element;
    } else {
      // If a regular HTMLElement is passed, replace it with our web component
      const webComponent = new TobogganErrorDisplay();
      element.parentNode?.replaceChild(webComponent, element);
      this.element = webComponent;
    }

    // Apply configuration
    if (config.autoHideDuration !== undefined) {
      this.element.setAttribute("auto-hide-duration", config.autoHideDuration.toString());
    }
    if (config.logToConsole !== undefined) {
      this.element.setAttribute("log-to-console", config.logToConsole.toString());
    }
  }

  /**
   * Show an error message
   */
  public show(message: string, error?: Error): void {
    this.element.show(message, error);
  }

  /**
   * Hide the error message
   */
  public hide(): void {
    this.element.hide();
  }

  /**
   * Check if error is currently visible
   */
  public get isVisible(): boolean {
    return this.element.isVisible;
  }

  /**
   * Dispose of the component
   */
  public dispose(): void {
    // The web component handles cleanup in disconnectedCallback
  }
}

// Register the custom element
if (!customElements.get("toboggan-error-display")) {
  customElements.define("toboggan-error-display", TobogganErrorDisplay);
}
