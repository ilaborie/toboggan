import navigationCss from "./navigation.css?raw";

/**
 * Navigation Web Component
 * Native web component that handles presentation navigation controls
 */

import { type ConnectionStatus, formatConnectionStatus } from "../app/communication";
import type { Command, Duration, StateState, Talk } from "../types";
import { COMMANDS } from "../utils/constants";
import { getRequireElement, renderContent } from "../utils/dom";
import { elapsed } from "../utils/duration";

export type NavigationCommandEvent = CustomEvent<Command>;

const BUTTONS = [
  {
    id: "first",
    label: "Go to first slide",
    title: "First slide",
    icon: "🏠",
    command: COMMANDS.FIRST,
  },
  {
    id: "prev",
    label: "Go to previous slide",
    title: "Previous slide",
    icon: "⬅️",
    command: COMMANDS.PREVIOUS,
  },
  { id: "next", label: "Go to next slide", title: "Next slide", icon: "➡️", command: COMMANDS.NEXT },
  {
    id: "last",
    label: "Go to last slide",
    title: "Last slide",
    icon: "🏁",
    command: COMMANDS.LAST,
  },
  { id: "pause", label: "Pause presentation", title: "Pause", icon: "⏸️", command: COMMANDS.PAUSE },
  {
    id: "resume",
    label: "Resume presentation",
    title: "Resume",
    icon: "▶️",
    command: COMMANDS.RESUME,
  },
  {
    id: "blink",
    label: "Blink",
    title: "Blink",
    icon: "🛎️",
    command: COMMANDS.BLINK,
  },
];

/**
 * Custom element for navigation controls with shadow DOM encapsulation
 */
export class TobogganNavigationElement extends HTMLElement {
  declare root: ShadowRoot;

  private progressElement!: HTMLProgressElement;
  private navigationElement!: HTMLElement;
  private _state: StateState | null = null;
  public get state(): StateState | null {
    return this._state;
  }
  public set state(value: StateState | null) {
    this._state = value;
    this.navigationElement.className = value ?? "none";
  }

  private talkElement!: HTMLElement;
  private _talk: Talk | null = null;
  public get talk(): Talk | null {
    return this._talk;
  }
  public set talk(value: Talk) {
    this._talk = value;
    if (this.talkElement) {
      this.talkElement.innerHTML = renderContent(value.title);
    }
  }

  private connectionStatusElement!: HTMLElement;
  private _connectionStatus: ConnectionStatus = { status: "closed" };
  public get connectionStatus(): ConnectionStatus {
    return this._connectionStatus;
  }
  public set connectionStatus(value: ConnectionStatus) {
    this._connectionStatus = value;
    if (this.connectionStatusElement) {
      this.connectionStatusElement.className = value.status ?? "none";
      this.connectionStatusElement.textContent = formatConnectionStatus(value);
    }
  }

  private slideCurrentElement!: HTMLElement;
  private _slideCurrent: number | null = null;
  public get slideCurrent(): number | null {
    return this._slideCurrent;
  }
  public set slideCurrent(value: number | null) {
    this._slideCurrent = value;
    if (this.slideCurrentElement) {
      this.slideCurrentElement.textContent = value?.toString() ?? "-";
    }
    if (value && this.progressElement) {
      this.progressElement.value = value;
    }
  }

  private slideCountElement!: HTMLElement;
  private _slideCount: number | null = null;
  public get slideCount(): number | null {
    return this._slideCount;
  }
  public set slideCount(value: number | null) {
    this._slideCount = value;
    if (this.slideCountElement) {
      this.slideCountElement.textContent = value?.toString() ?? "-";
    }
    if (value && this.progressElement) {
      this.progressElement.max = value;
    }
  }

  private durationElement!: HTMLElement;
  private _duration: Duration | null = null;
  private interval: number | null = null;
  public get duration(): Duration | null {
    return this._duration;
  }
  public set duration(value: Duration | null) {
    this._duration = value;

    if (this.interval !== null) {
      clearInterval(this.interval);
      this.interval = null;
    }

    if (value) {
      this.interval = setInterval(() => {
        if (this._duration && this.durationElement) {
          this._duration.secs += 1;
          this.durationElement.textContent = elapsed(this._duration);
        }
      }, 1000);
    } else if (this.durationElement) {
      this.durationElement.textContent = "";
    }
  }

  private listener: EventListener | null = null;

  constructor() {
    super();
    this.root = this.attachShadow({ mode: "open" });

    const style = document.createElement("style");
    style.textContent = navigationCss;
    this.root.appendChild(style);
  }

  connectedCallback(): void {
    this.navigationElement = document.createElement("nav");
    this.navigationElement.dataset.theme = "dark";

    const buttons = BUTTONS.map(
      ({ id, label, title, icon, command }) =>
        `<button id="${id}-btn" aria-label="${label}" title="${title}" data-command="${command.command}">${icon}</button>`
    ).join("\n ");

    this.navigationElement.innerHTML = `
<progress value="0"></progress>
<h1></h1>
<div class="buttons">${buttons}</div>
<div class="status">
  <span class="connection"></span>
  <span class="state"></span>
  <div>
    <span class="slide"></span>
    <span class="count"></span>
  </div>
  <span class="duration"></span>
</div>
`;

    this.root.appendChild(this.navigationElement);
    this.talkElement = getRequireElement("h1", this.navigationElement);
    this.connectionStatusElement = getRequireElement(".connection", this.navigationElement);
    this.progressElement = getRequireElement("progress", this.navigationElement);
    this.slideCurrentElement = getRequireElement(".slide", this.navigationElement);
    this.slideCountElement = getRequireElement(".count", this.navigationElement);
    this.durationElement = getRequireElement(".duration", this.navigationElement);
    this.listener = (event) => {
      const commandName = (event.target as HTMLElement).getAttribute("data-command");
      if (commandName) {
        const command = BUTTONS.find((b) => b.command.command === commandName)?.command;
        if (command) {
          // console.log("📡 command", command);
          this.dispatchEvent(new CustomEvent("command", { detail: command }));
        }
      }
    };
    this.navigationElement.addEventListener("click", this.listener);
  }

  disconnectedCallback(): void {
    if (this.navigationElement) {
      if (this.listener) {
        this.navigationElement.removeEventListener("click", this.listener);
      }
      this.root.removeChild(this.navigationElement);
    }
  }
}

// Register the custom element
if (!customElements.get("toboggan-navigation")) {
  customElements.define("toboggan-navigation", TobogganNavigationElement);
}

declare global {
  interface HTMLElementTagNameMap {
    "toboggan-navigation": TobogganNavigationElement;
  }

  interface HTMLElementEventMap {
    command: NavigationCommandEvent;
  }
}
