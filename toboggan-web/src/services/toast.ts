/**
 * Toast Service and Container Web Component
 * Displays temporary notifications for errors, status changes, and information
 */

import { type ToastType, TobogganToast } from "../components/toast.js";

export type { ToastType };

export interface ToastConfig {
  duration?: number;
  persistent?: boolean;
  type?: ToastType;
}

export interface ToastMessage {
  id: string;
  message: string;
  type: ToastType;
  timestamp: number;
  persistent: boolean;
  element?: TobogganToast;
}

/**
 * Custom element for the toast container
 */
export class TobogganToastContainer extends HTMLElement {
  declare root: ShadowRoot;
  private container!: HTMLDivElement;
  private messages = new Map<string, ToastMessage>();
  // biome-ignore lint/correctness/noUnusedPrivateClassMembers: Used in show method for ID generation
  private messageCounter = 0;

  constructor() {
    super();

    // Create shadow DOM
    this.root = this.attachShadow({ mode: "open" });

    // Create the internal structure
    this.createStructure();
    this.applyStyles();
    this.setupEventListeners();
  }

  /**
   * Show an error toast
   */
  public error(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.show(message, { ...config, type: "error" });
  }

  /**
   * Show a warning toast
   */
  public warning(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.show(message, { ...config, type: "warning" });
  }

  /**
   * Show an info toast
   */
  public info(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.show(message, { ...config, type: "info" });
  }

  /**
   * Show a success toast
   */
  public success(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.show(message, { ...config, type: "success" });
  }

  /**
   * Show a toast message
   */
  public show(message: string, config: ToastConfig = {}): string {
    const id = `toast-${++this.messageCounter}`;
    const toast: ToastMessage = {
      id,
      message,
      type: config.type ?? "info",
      timestamp: Date.now(),
      persistent: config.persistent ?? false,
    };

    // Create toast element
    const toastElement = new TobogganToast();
    toastElement.type = toast.type;
    toastElement.message = toast.message;
    toastElement.persistent = toast.persistent;

    if (config.duration !== undefined) {
      toastElement.duration = config.duration;
    }

    // Set unique ID for tracking
    toastElement.id = id;
    toast.element = toastElement;

    // Add to container
    this.container.appendChild(toastElement);

    // Store message
    this.messages.set(id, toast);

    // Log to console for debugging
    console.log(`[Toast ${toast.type.toUpperCase()}] ${message}`);

    return id;
  }

  /**
   * Hide a specific toast
   */
  public hide(id: string): void {
    const toast = this.messages.get(id);
    if (toast?.element) {
      toast.element.dismiss();
      this.messages.delete(id);
    }
  }

  /**
   * Clear all toasts
   */
  public clear(): void {
    this.messages.forEach((toast) => {
      if (toast.element) {
        toast.element.dismiss();
      }
    });
    this.messages.clear();
  }

  /**
   * Get connection status message
   */
  public showConnectionStatus(
    status: "connecting" | "connected" | "disconnected" | "reconnecting",
    details?: string
  ): string {
    const statusMessages = {
      connecting: "Connecting to server...",
      connected: "Connected to server",
      disconnected: "Disconnected from server",
      reconnecting: "Reconnecting to server...",
    };

    const message = details ? `${statusMessages[status]}: ${details}` : statusMessages[status];
    const type: ToastType =
      status === "connected" ? "success" : status === "disconnected" ? "error" : "info";

    return this.show(message, {
      type,
      duration: status === "connected" ? 2000 : 4000,
    });
  }

  /**
   * Show presentation status change
   */
  public showPresentationStatus(status: "running" | "paused" | "done"): string {
    const statusMessages = {
      running: "Presentation started",
      paused: "Presentation paused",
      done: "Presentation completed",
    };

    const type: ToastType = status === "done" ? "success" : "info";
    return this.show(statusMessages[status], { type, duration: 2000 });
  }

  /**
   * Create the internal DOM structure
   */
  private createStructure(): void {
    this.container = document.createElement("div");
    this.container.className = "toast-container";
    this.root.appendChild(this.container);
  }

  /**
   * Apply styles to the shadow DOM
   */
  private applyStyles(): void {
    const style = document.createElement("style");
    style.textContent = `
      :host {
        position: fixed;
        top: 20px;
        right: 20px;
        z-index: 1000;
        max-width: 400px;
        pointer-events: none;
        display: block;
      }

      :host([hidden]) {
        display: none !important;
      }

      .toast-container {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        pointer-events: auto;
      }
    `;

    this.root.appendChild(style);
  }

  /**
   * Setup event listeners
   */
  private setupEventListeners(): void {
    // Listen for toast dismiss events to clean up our tracking
    this.addEventListener("toast-dismiss", (event) => {
      const toastElement = event.target as TobogganToast;

      if (toastElement?.id) {
        this.messages.delete(toastElement.id);
      }
    });
  }
}

export class ToastService {
  private readonly container: TobogganToastContainer;

  constructor(containerId: string = "toast-container") {
    // Find existing container or create a new one
    let container = document.getElementById(containerId);

    if (!container || !(container instanceof TobogganToastContainer)) {
      // Create new container
      const newContainer = new TobogganToastContainer();
      newContainer.id = containerId;

      // If there was an old element, replace it
      if (container?.parentNode) {
        container.parentNode.replaceChild(newContainer, container);
      } else {
        // Append to body if no existing element
        document.body.appendChild(newContainer);
      }
      container = newContainer;
    }

    this.container = container as TobogganToastContainer;
  }

  /**
   * Show an error toast
   */
  public error(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.container.error(message, config);
  }

  /**
   * Show a warning toast
   */
  public warning(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.container.warning(message, config);
  }

  /**
   * Show an info toast
   */
  public info(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.container.info(message, config);
  }

  /**
   * Show a success toast
   */
  public success(message: string, config: Omit<ToastConfig, "type"> = {}): string {
    return this.container.success(message, config);
  }

  /**
   * Show a toast message
   */
  public show(message: string, config: ToastConfig = {}): string {
    return this.container.show(message, config);
  }

  /**
   * Hide a specific toast
   */
  public hide(id: string): void {
    this.container.hide(id);
  }

  /**
   * Clear all toasts
   */
  public clear(): void {
    this.container.clear();
  }

  /**
   * Get connection status message
   */
  public showConnectionStatus(
    status: "connecting" | "connected" | "disconnected" | "reconnecting",
    details?: string
  ): string {
    return this.container.showConnectionStatus(status, details);
  }

  /**
   * Show presentation status change
   */
  public showPresentationStatus(status: "running" | "paused" | "done"): string {
    return this.container.showPresentationStatus(status);
  }
}

// Register the custom elements
if (!customElements.get("toboggan-toast-container")) {
  customElements.define("toboggan-toast-container", TobogganToastContainer);
}
