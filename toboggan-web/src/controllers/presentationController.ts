/**
 * Presentation Controller
 * Coordinates the presentation state and services
 */

import { SlideRenderer } from "../services/slideRenderer.js";
import { SlidesApiService } from "../services/slidesApi.js";
import { TimerService } from "../services/timer.js";
import { type CommunicationCallbacks, CommunicationService } from "../services/websocket.js";
import type {
  AppConfig,
  Command,
  ConnectionStatus,
  Notification,
  SlideId,
  State,
} from "../types.js";
// Note: showError is deprecated, will be replaced by ErrorComponent

export interface PresentationElements {
  connectionStatus: HTMLElement;
  slideCounter: HTMLElement;
  durationDisplay: HTMLElement;
  errorDisplay: HTMLElement;
  appElement: HTMLElement;
}

export class PresentationController {
  private readonly elements: PresentationElements;
  private readonly communicationService: CommunicationService;
  private readonly slidesApi: SlidesApiService;
  private readonly slideRenderer: SlideRenderer;
  private readonly timer: TimerService;

  private currentSlide: SlideId | null = null;

  constructor(clientId: string, config: AppConfig, elements: PresentationElements) {
    this.elements = elements;

    // Initialize services
    this.slidesApi = new SlidesApiService(config.apiBaseUrl);
    this.slideRenderer = new SlideRenderer(elements.appElement);
    this.timer = new TimerService(elements.durationDisplay);

    // Initialize communication service with callbacks
    const callbacks: CommunicationCallbacks = {
      onOpen: () => this.handleWebSocketOpen(),
      onNotification: (notification) => this.handleNotification(notification),
      onClose: () => this.handleWebSocketClose(),
      onError: (error) => this.handleError(error),
      onReconnecting: (attempt, max, delay) => this.handleReconnecting(attempt, max, delay),
      onMaxRetriesReached: () => this.handleMaxRetriesReached(),
      onLatencyUpdate: (latency) => this.handleLatencyUpdate(latency),
    };

    this.communicationService = new CommunicationService(clientId, config.websocket, callbacks);
  }

  /**
   * Start the presentation controller
   */
  public start(): void {
    this.updateConnectionStatus("Connecting...", "connecting");
    this.communicationService.connect();
  }

  /**
   * Send a navigation command
   */
  public sendCommand(command: Command): void {
    this.communicationService.sendCommand(command);
  }

  /**
   * Dispose of all resources
   */
  public dispose(): void {
    this.timer.stop();
    this.communicationService.dispose();
  }

  private handleWebSocketOpen(): void {
    this.updateConnectionStatus("Connected", "connected");
    this.communicationService.register();
  }

  private handleWebSocketClose(): void {
    this.updateConnectionStatus("Disconnected", "disconnected");
    this.slidesApi.clearCache();
    this.timer.stop();
  }

  private handleReconnecting(attempt: number, max: number, delaySeconds: number): void {
    this.updateConnectionStatus(
      `Reconnecting in ${delaySeconds}s... (${attempt}/${max})`,
      "reconnecting"
    );
  }

  private handleMaxRetriesReached(): void {
    this.handleError("Max reconnection attempts reached. Please refresh the page.");
  }

  private handleLatencyUpdate(latency: number): void {
    // TODO: Display latency in UI or use for diagnostics
    console.log(`Connection latency: ${latency}ms`);
  }

  private handleError(message: string): void {
    // TODO: Replace with ErrorComponent injection
    console.error("PresentationController error:", message);
    this.elements.errorDisplay.textContent = message;
    this.elements.errorDisplay.style.display = "block";

    // Auto-hide after 5 seconds
    setTimeout(() => {
      this.elements.errorDisplay.style.display = "none";
    }, 5000);
  }

  private handleNotification(notification: Notification): void {
    switch (notification.type) {
      case "State":
        this.handleStateNotification(notification.state);
        break;
      case "Error":
        this.handleError(notification.message);
        break;
      case "Pong":
        // Latency calculation is handled in the communication service
        break;
      default:
        console.warn("Unknown notification type:", (notification as { type?: string }).type);
    }
  }

  private async handleStateNotification(state: State): Promise<void> {
    // Update timer state
    this.timer.updateState(state);

    // Update current slide
    if (state.state === "Running") {
      this.currentSlide = state.current;
      this.updateConnectionStatus("Running", "running");
    } else if (state.state === "Paused") {
      this.currentSlide = state.current;
      this.updateConnectionStatus("Paused", "paused");
    } else if (state.state === "Done") {
      this.currentSlide = state.current;
      this.updateConnectionStatus("Done", "done");
    }

    // Load and display slide
    await this.loadCurrentSlide();
  }

  private async loadCurrentSlide(): Promise<void> {
    if (this.currentSlide === null) return;

    try {
      const slide = await this.slidesApi.getSlide(this.currentSlide);
      this.slideRenderer.displaySlide(slide);
      this.updateSlideCounter();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      console.error("Failed to load slide:", error);
      this.handleError(`Failed to load slide: ${errorMessage}`);
    }
  }

  private updateSlideCounter(): void {
    if (this.currentSlide !== null) {
      const displayNumber = this.slidesApi.getSlideDisplayNumber(this.currentSlide);
      const total = this.slidesApi.getTotalSlides();
      const totalStr = total ? ` / ${total}` : "";
      this.elements.slideCounter.textContent = `Slide: ${displayNumber}${totalStr}`;
    } else {
      this.elements.slideCounter.textContent = "Slide: - / -";
    }
  }

  private updateConnectionStatus(status: string, className: ConnectionStatus): void {
    this.elements.connectionStatus.textContent = status;
    this.elements.connectionStatus.className = className;
  }
}
