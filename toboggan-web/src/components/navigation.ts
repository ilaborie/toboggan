/**
 * Navigation Web Component
 * Native web component that handles presentation navigation controls
 */

import type { Command } from "../types.js";
import { COMMANDS } from "../constants/index.js";

export interface NavigationState {
  connectionStatus?: string;
  slideCounter?: string;
  duration?: string;
  presentationTitle?: string;
}

/**
 * Custom element for navigation controls with shadow DOM encapsulation
 */
export class TobogganNavigation extends HTMLElement {
  declare root: ShadowRoot;
  private controls!: HTMLDivElement;
  private status!: HTMLDivElement;
  private titleElement!: HTMLHeadingElement;
  private connectionStatusElement!: HTMLSpanElement;
  private slideCounterElement!: HTMLSpanElement;
  private durationElement!: HTMLSpanElement;

  constructor() {
    super();

    // Create shadow DOM
    this.root = this.attachShadow({ mode: "open" });

    // Create the internal structure
    this.createStructure();
    this.applyStyles();
    this.attachEventListeners();
  }

  /**
   * Observed attributes for the web component
   */
  static get observedAttributes(): string[] {
    return [
      "presentation-title",
      "connection-status", 
      "slide-counter",
      "duration"
    ];
  }

  /**
   * Handle attribute changes
   */
  attributeChangedCallback(name: string, _oldValue: string | null, newValue: string | null): void {
    switch (name) {
      case "presentation-title":
        this.titleElement.textContent = newValue || "Toboggan Presentation";
        break;
      case "connection-status":
        this.connectionStatusElement.textContent = newValue || "Disconnected";
        break;
      case "slide-counter":
        this.slideCounterElement.textContent = newValue || "Slide: - / -";
        break;
      case "duration":
        this.durationElement.textContent = newValue || "Duration: --:--:--";
        break;
    }
  }

  /**
   * Update the navigation state
   */
  public updateState(state: NavigationState): void {
    if (state.connectionStatus !== undefined) {
      this.setAttribute("connection-status", state.connectionStatus);
    }
    if (state.slideCounter !== undefined) {
      this.setAttribute("slide-counter", state.slideCounter);
    }
    if (state.duration !== undefined) {
      this.setAttribute("duration", state.duration);
    }
    if (state.presentationTitle !== undefined) {
      this.setAttribute("presentation-title", state.presentationTitle);
    }
  }

  /**
   * Create the internal DOM structure
   */
  private createStructure(): void {
    // Create nav element
    const nav = document.createElement("nav");
    nav.setAttribute("role", "navigation");
    nav.setAttribute("aria-label", "Presentation controls");

    // Create title
    this.titleElement = document.createElement("h1");
    this.titleElement.textContent = this.getAttribute("presentation-title") || "Toboggan Presentation";

    // Create controls container
    this.controls = document.createElement("div");
    this.controls.className = "controls";

    // Create navigation buttons
    const buttons = [
      { id: "first", label: "Go to first slide", title: "First slide", icon: "🏠", command: COMMANDS.FIRST },
      { id: "prev", label: "Go to previous slide", title: "Previous slide", icon: "⬅️", command: COMMANDS.PREVIOUS },
      { id: "next", label: "Go to next slide", title: "Next slide", icon: "➡️", command: COMMANDS.NEXT },
      { id: "last", label: "Go to last slide", title: "Last slide", icon: "🏁", command: COMMANDS.LAST },
      { id: "pause", label: "Pause presentation", title: "Pause", icon: "⏸️", command: COMMANDS.PAUSE },
      { id: "resume", label: "Resume presentation", title: "Resume", icon: "▶️", command: COMMANDS.RESUME },
    ];

    buttons.forEach(({ id, label, title, icon, command }) => {
      const button = document.createElement("button");
      button.id = `${id}-btn`;
      button.setAttribute("aria-label", label);
      button.setAttribute("title", title);
      button.textContent = icon;
      button.dataset.command = JSON.stringify(command);
      this.controls.appendChild(button);
    });

    // Create status container
    this.status = document.createElement("div");
    this.status.className = "status";

    this.connectionStatusElement = document.createElement("span");
    this.connectionStatusElement.id = "connection-status";
    this.connectionStatusElement.setAttribute("aria-live", "polite");
    this.connectionStatusElement.textContent = this.getAttribute("connection-status") || "Connecting...";

    this.slideCounterElement = document.createElement("span");
    this.slideCounterElement.id = "slide-counter";
    this.slideCounterElement.setAttribute("aria-live", "polite");
    this.slideCounterElement.textContent = this.getAttribute("slide-counter") || "Slide: - / -";

    this.durationElement = document.createElement("span");
    this.durationElement.id = "duration-display";
    this.durationElement.setAttribute("aria-live", "polite");
    this.durationElement.textContent = this.getAttribute("duration") || "Duration: --:--:--";

    this.status.appendChild(this.connectionStatusElement);
    this.status.appendChild(this.slideCounterElement);
    this.status.appendChild(this.durationElement);

    // Assemble the nav
    nav.appendChild(this.titleElement);
    nav.appendChild(this.controls);
    nav.appendChild(this.status);

    this.root.appendChild(nav);
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

      nav {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 1rem;
        background: var(--pico-background-color, #fff);
        border-bottom: 1px solid var(--pico-border-color, #dee2e6);
        gap: 1rem;
        flex-wrap: wrap;
      }

      h1 {
        margin: 0;
        font-size: 1.5rem;
        color: var(--pico-color, #212529);
        flex-shrink: 0;
      }

      .controls {
        display: flex;
        gap: 0.5rem;
        align-items: center;
        flex-wrap: wrap;
      }

      .controls button {
        background: var(--pico-primary-background, #0d6efd);
        color: var(--pico-primary-color, #fff);
        border: none;
        border-radius: var(--pico-border-radius, 0.375rem);
        padding: 0.5rem;
        font-size: 1.2rem;
        cursor: pointer;
        transition: all 0.2s ease;
        min-width: 40px;
        height: 40px;
        display: flex;
        align-items: center;
        justify-content: center;
      }

      .controls button:hover {
        background: var(--pico-primary-hover-background, #0b5ed7);
        transform: translateY(-1px);
      }

      .controls button:active {
        transform: translateY(0);
      }

      .controls button:focus {
        outline: 2px solid var(--pico-primary-focus, #86b7fe);
        outline-offset: 2px;
      }

      .status {
        display: flex;
        gap: 1rem;
        align-items: center;
        font-size: 0.9rem;
        color: var(--pico-muted-color, #6c757d);
        flex-wrap: wrap;
      }

      .status span {
        white-space: nowrap;
      }

      @media (max-width: 768px) {
        nav {
          flex-direction: column;
          align-items: stretch;
          gap: 0.5rem;
        }

        h1 {
          text-align: center;
          font-size: 1.25rem;
        }

        .controls {
          justify-content: center;
        }

        .status {
          justify-content: center;
          font-size: 0.8rem;
          gap: 0.5rem;
        }
      }
    `;

    this.root.appendChild(style);
  }

  /**
   * Attach event listeners to navigation buttons
   */
  private attachEventListeners(): void {
    this.controls.addEventListener("click", (event) => {
      const target = event.target as HTMLElement;
      if (target.tagName === "BUTTON" && target.dataset.command) {
        try {
          const command: Command = JSON.parse(target.dataset.command);
          this.dispatchEvent(
            new CustomEvent("navigation-command", {
              detail: { command },
              bubbles: true,
            })
          );
        } catch (error) {
          console.error("Failed to parse command:", error);
        }
      }
    });
  }

  /**
   * Get button element by command type
   */
  public getButton(commandType: string): HTMLButtonElement | null {
    const buttons = this.controls.querySelectorAll("button");
    for (const button of buttons) {
      if (button.dataset.command) {
        try {
          const command = JSON.parse(button.dataset.command);
          if (command.command === commandType) {
            return button;
          }
        } catch {
          // Invalid command data, skip
        }
      }
    }
    return null;
  }

  /**
   * Enable or disable a specific button
   */
  public setButtonEnabled(commandType: string, enabled: boolean): void {
    const button = this.getButton(commandType);
    if (button) {
      button.disabled = !enabled;
    }
  }

  /**
   * Enable or disable all buttons
   */
  public setAllButtonsEnabled(enabled: boolean): void {
    const buttons = this.controls.querySelectorAll("button");
    buttons.forEach((button) => {
      (button as HTMLButtonElement).disabled = !enabled;
    });
  }
}

// Register the custom element
if (!customElements.get("toboggan-navigation")) {
  customElements.define("toboggan-navigation", TobogganNavigation);
}