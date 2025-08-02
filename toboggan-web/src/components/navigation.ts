import navigationCss from "./navigation.css?raw";

/**
 * Navigation Web Component
 * Native web component that handles presentation navigation controls
 */

import type { Command, Duration, StateState } from "../types";
import { COMMANDS } from "../constants.js";
import { elapsed } from "../utils/duration.js";
import { ConnectionStatus } from "../services/communication.js";

export interface NavigationState {
  connectionStatus?: string;
  slideCounter?: string;
  duration?: string;
  presentationTitle?: string;
}

export type NaviationCommandEvent = CustomEvent<Command>;

const BUTTONS = [
  { id: "first", label: "Go to first slide", title: "First slide", icon: "🏠", command: COMMANDS.FIRST },
  { id: "prev", label: "Go to previous slide", title: "Previous slide", icon: "⬅️", command: COMMANDS.PREVIOUS },
  { id: "next", label: "Go to next slide", title: "Next slide", icon: "➡️", command: COMMANDS.NEXT },
  { id: "last", label: "Go to last slide", title: "Last slide", icon: "🏁", command: COMMANDS.LAST },
  { id: "pause", label: "Pause presentation", title: "Pause", icon: "⏸️", command: COMMANDS.PAUSE },
  { id: "resume", label: "Resume presentation", title: "Resume", icon: "▶️", command: COMMANDS.RESUME },
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
    this.navigationElement.className = value ?? 'none';
  }

  private talkElement!: HTMLElement;
  private _talk: string = "";
  public get talk(): string {
    return this._talk;
  }
  public set talk(value: string) {
    console.log('👋', { talk: value });
    this._talk = value;
    if (this.talkElement) {
      this.talkElement.className = value ?? 'none';
      this.talkElement.textContent = value;
    }
  }

  private connectionStatusElement!: HTMLElement;
  private _connectionStatus: ConnectionStatus = "paused";
  public get connectionStatus(): ConnectionStatus {
    return this._connectionStatus;
  }
  public set connectionStatus(value: ConnectionStatus) {
    console.log('👋', { connectionStatus: value });
    this._connectionStatus = value;
    if (this.connectionStatusElement) {
      this.connectionStatusElement.className = value ?? 'none';
      this.connectionStatusElement.textContent = value;
    }
  }

  private slideCurrentElement!: HTMLElement;
  private _slideCurrent: number | null = null;
  public get slideCurrent(): number | null {
    return this._slideCurrent;
  }
  public set slideCurrent(value: number | null) {
    console.log('👋', { current: value });
    this._slideCurrent = value;
    if (this.slideCurrentElement) {
      this.slideCurrentElement.textContent = value?.toString() ?? '-';
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
    console.log('👋', { count: value });
    this._slideCount = value;
    if (this.slideCountElement) {
      this.slideCountElement.textContent = value?.toString() ?? '-';
    }
    if (value && this.progressElement) {
      this.progressElement.max = value;
    }
  }

  private durationElement!: HTMLElement;
  private _duration: Duration | null = null;
  public get duration(): Duration | null {
    return this._duration;
  }
  public set duration(value: Duration | null) {
    console.log('👋', { duration: value });
    this._duration = value;
    if (this.durationElement) {
      if (value) {
        this.durationElement.textContent = elapsed(value);
      } else {
        this.durationElement.textContent = '';
      }
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

    const buttons = BUTTONS.map(({ id, label, title, icon, command }) =>
      `<button id="${id}-btn" aria-label="${label}" title="${title}" data-command="${command.command}">${icon}</button>`
    ).join('\n ');

    this.navigationElement.innerHTML = `
<progress></progress>
<h1></h1>
<div class="buttons>${buttons}</div>
<div class="status">
  <span class="connection"></span>
  <span class="state"></span>
  <div>
    <span class="slide"></span>
    <span class="count"></span>
  </div>
  <span class="duration"></duration>
</div>
`;
    this.root.appendChild(this.navigationElement);
    this.talkElement = this.navigationElement.querySelector("h1")!;
    this.connectionStatusElement = this.navigationElement.querySelector(".connection")!;
    this.progressElement = this.navigationElement.querySelector("progress")!;
    this.slideCurrentElement = this.navigationElement.querySelector(".slide")!;
    this.slideCountElement = this.navigationElement.querySelector(".count")!;
    this.durationElement = this.navigationElement.querySelector(".duration")!;
    this.listener = (event) => {
      const command = (event.target as HTMLElement).getAttribute("data-command");
      if (command) {
        console.log('📡 command', command);
        this.dispatchEvent(new CustomEvent('command', { detail: command }));
      }
    };
    this.navigationElement.addEventListener('click', this.listener);
  }

  disconnectedCallback(): void {
    if (this.navigationElement) {
      if (this.listener) {
        this.navigationElement.removeEventListener('click', this.listener);
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
}