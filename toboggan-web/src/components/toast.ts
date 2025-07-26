/**
 * Toast Web Component
 * Native web component for individual toast notifications with shadow DOM encapsulation
 */

export type ToastType = "error" | "warning" | "info" | "success";

/**
 * Custom element for individual toast messages
 */
export class TobogganToast extends HTMLElement {
  declare shadowRoot: ShadowRoot;
  private toastContainer!: HTMLDivElement;
  private closeButton!: HTMLButtonElement;
  private autoHideTimeout: number | null = null;

  constructor() {
    super();

    // Create shadow DOM
    this.shadowRoot = this.attachShadow({ mode: "open" });

    // Create the internal structure
    this.createStructure();
    this.applyStyles();
    this.setupEventListeners();
  }

  /**
   * Observed attributes for the web component
   */
  static get observedAttributes(): string[] {
    return ["type", "message", "duration", "persistent", "auto-hide"];
  }

  /**
   * Called when the element is connected to the DOM
   */
  connectedCallback(): void {
    this.updateContent();
    this.setupAutoHide();
    this.animateIn();
  }

  /**
   * Called when the element is disconnected from the DOM
   */
  disconnectedCallback(): void {
    this.clearAutoHideTimeout();
  }

  /**
   * Handle attribute changes
   */
  attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {
    if (oldValue === newValue) return;

    switch (name) {
      case "type":
        this.updateType();
        break;
      case "message":
        this.updateMessage();
        break;
      case "duration":
      case "persistent":
      case "auto-hide":
        this.setupAutoHide();
        break;
    }
  }

  /**
   * Get the toast type
   */
  public get type(): ToastType {
    return (this.getAttribute("type") as ToastType) || "info";
  }

  /**
   * Set the toast type
   */
  public set type(value: ToastType) {
    this.setAttribute("type", value);
  }

  /**
   * Get the toast message
   */
  public get message(): string {
    return this.getAttribute("message") || "";
  }

  /**
   * Set the toast message
   */
  public set message(value: string) {
    this.setAttribute("message", value);
  }

  /**
   * Get the duration in milliseconds
   */
  public get duration(): number | null {
    const duration = this.getAttribute("duration");
    return duration ? parseInt(duration, 10) : null;
  }

  /**
   * Set the duration in milliseconds
   */
  public set duration(value: number | null) {
    if (value === null) {
      this.removeAttribute("duration");
    } else {
      this.setAttribute("duration", value.toString());
    }
  }

  /**
   * Check if toast is persistent (doesn't auto-hide)
   */
  public get persistent(): boolean {
    return this.hasAttribute("persistent");
  }

  /**
   * Set persistent state
   */
  public set persistent(value: boolean) {
    if (value) {
      this.setAttribute("persistent", "");
    } else {
      this.removeAttribute("persistent");
    }
  }

  /**
   * Dismiss the toast with animation
   */
  public dismiss(): void {
    this.clearAutoHideTimeout();

    // Dispatch dismiss event
    this.dispatchEvent(
      new CustomEvent("toast-dismiss", {
        detail: { type: this.type, message: this.message },
        bubbles: true,
      })
    );

    // Animate out
    this.toastContainer.style.transform = "translateX(100%)";
    this.toastContainer.style.opacity = "0";

    // Remove after animation
    setTimeout(() => {
      if (this.parentNode) {
        this.parentNode.removeChild(this);
      }
    }, 300);
  }

  /**
   * Create the internal DOM structure
   */
  private createStructure(): void {
    this.toastContainer = document.createElement("div");
    this.toastContainer.className = "toast-container";
    this.toastContainer.setAttribute("role", "alert");
    this.toastContainer.setAttribute("aria-live", "polite");

    const messageElement = document.createElement("span");
    messageElement.className = "toast-message";

    this.closeButton = document.createElement("button");
    this.closeButton.className = "toast-close";
    this.closeButton.setAttribute("aria-label", "Close notification");
    this.closeButton.innerHTML = "&times;";

    this.toastContainer.appendChild(messageElement);
    this.toastContainer.appendChild(this.closeButton);
    this.shadowRoot.appendChild(this.toastContainer);
  }

  /**
   * Apply styles to the shadow DOM
   */
  private applyStyles(): void {
    const style = document.createElement("style");
    style.textContent = `
      :host {
        display: block;
        margin-bottom: 8px;
        max-width: 400px;
      }
      
      :host([hidden]) {
        display: none !important;
      }

      .toast-container {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 16px;
        border-radius: 4px;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
        font-family: system-ui, -apple-system, sans-serif;
        font-size: 14px;
        line-height: 1.4;
        word-wrap: break-word;
        transform: translateX(100%);
        opacity: 0;
        transition: all 0.3s ease-in-out;
        cursor: pointer;
      }

      .toast-container.show {
        transform: translateX(0);
        opacity: 1;
      }

      .toast-message {
        flex: 1;
        margin-right: 12px;
      }

      .toast-close {
        background: none;
        border: none;
        color: inherit;
        font-size: 18px;
        line-height: 1;
        opacity: 0.7;
        cursor: pointer;
        padding: 0;
        width: 20px;
        height: 20px;
        display: flex;
        align-items: center;
        justify-content: center;
      }

      .toast-close:hover {
        opacity: 1;
      }

      .toast-close:focus {
        outline: 2px solid currentColor;
        outline-offset: 2px;
      }

      /* Type-specific styles */
      :host([type="error"]) .toast-container {
        background-color: #f56565;
        color: #ffffff;
      }

      :host([type="warning"]) .toast-container {
        background-color: #ed8936;
        color: #ffffff;
      }

      :host([type="info"]) .toast-container {
        background-color: #4299e1;
        color: #ffffff;
      }

      :host([type="success"]) .toast-container {
        background-color: #48bb78;
        color: #ffffff;
      }
    `;

    this.shadowRoot.appendChild(style);
  }

  /**
   * Setup event listeners
   */
  private setupEventListeners(): void {
    // Close button click
    this.closeButton.addEventListener("click", (e) => {
      e.stopPropagation();
      this.dismiss();
    });

    // Container click to dismiss
    this.toastContainer.addEventListener("click", () => {
      this.dismiss();
    });

    // Keyboard accessibility
    this.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        this.dismiss();
      }
    });
  }

  /**
   * Update the toast content based on attributes
   */
  private updateContent(): void {
    this.updateMessage();
    this.updateType();
  }

  /**
   * Update the message content
   */
  private updateMessage(): void {
    const messageElement = this.shadowRoot.querySelector(".toast-message");
    if (messageElement) {
      messageElement.textContent = this.message;
    }
  }

  /**
   * Update the toast type styling
   */
  private updateType(): void {
    // Type styles are handled via CSS attribute selectors
    // No additional JavaScript needed here
  }

  /**
   * Setup auto-hide functionality
   */
  private setupAutoHide(): void {
    this.clearAutoHideTimeout();

    if (this.persistent || this.hasAttribute("persistent")) {
      return;
    }

    const duration = this.duration || this.getDefaultDuration(this.type);
    if (duration > 0) {
      this.autoHideTimeout = window.setTimeout(() => {
        this.dismiss();
      }, duration);
    }
  }

  /**
   * Clear auto-hide timeout
   */
  private clearAutoHideTimeout(): void {
    if (this.autoHideTimeout !== null) {
      clearTimeout(this.autoHideTimeout);
      this.autoHideTimeout = null;
    }
  }

  /**
   * Get default duration for toast type
   */
  private getDefaultDuration(type: ToastType): number {
    const durations = {
      error: 6000,
      warning: 5000,
      info: 3000,
      success: 2000,
    };
    return durations[type];
  }

  /**
   * Animate toast in
   */
  private animateIn(): void {
    // Use requestAnimationFrame to ensure the element is rendered
    requestAnimationFrame(() => {
      this.toastContainer.classList.add("show");
    });
  }
}

// Register the custom element
if (!customElements.get("toboggan-toast")) {
  customElements.define("toboggan-toast", TobogganToast);
}
