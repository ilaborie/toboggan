/**
 * Presentation Controller
 * Coordinates the presentation state and services
 */

import { PresentationElements } from "../elements.js";
import { SlideRenderer } from "../services/slideRenderer.js";
import { SlidesApiService } from "../services/slidesApi.js";
import { type CommunicationCallbacks, CommunicationService, ConnectionStatus } from "../services/communication.js";
import type {
  Command,
  SlideId,
  State,
} from "../types";
import { AppConfig } from "../config.js";

export class PresentationController {
  private readonly communicationService: CommunicationService;
  private readonly slidesApi: SlidesApiService;
  private readonly slideRenderer: SlideRenderer;

  private currentSlide: SlideId | null = null;

  constructor(
    clientId: string,
    config: AppConfig,
    private readonly elements: PresentationElements
  ) {

    // Initialize services
    this.slidesApi = new SlidesApiService(config.apiBaseUrl);
    this.slideRenderer = new SlideRenderer(elements.appElement);

    // Initialize communication service with callbacks
    const callbacks: CommunicationCallbacks = {
      onConnectionStatusChange: (status) => this.handConnectionStatus(status),
      onStateChange: (state) => this.handleStateNotification(state),
      onError: (error) => this.handleError(error),
      // TODO move in connection status
      onReconnecting: (attempt, max, delay) => this.handleReconnecting(attempt, max, delay),
      onMaxRetriesReached: () => this.handleMaxRetriesReached(),
      onLatencyUpdate: (latency) => this.handleLatencyUpdate(latency),
    };

    this.communicationService = new CommunicationService(clientId, config.websocket, callbacks);
  }

  private handConnectionStatus(status: ConnectionStatus) {
    this.elements.navigationElement.connectionStatus = status;
    this.elements.toastElement.toast("info", status);

    if (status === "connected") {
      this.communicationService.register();
    } else if (status === 'done') {
      this.slidesApi.clearCache();
    }
  }

  /**
   * Start the presentation controller
   */
  public start(): void {
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
    this.slidesApi.clearCache();
    this.communicationService.dispose();
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
    console.info(`Connection latency: ${latency}ms`);
  }

  private handleError(message: string): void {
    console.error("PresentationController error:", message);
    this.elements.toastElement.toast('error', message);
  }


  private async handleStateNotification(state: State): Promise<void> {
    this.elements.navigationElement.state = state.state;
    this.elements.navigationElement.slideCurrent = state.current;
    this.elements.navigationElement.duration = state.total_duration;

    if (state.state === 'Done') {
      this.elements.toastElement.toast('success', '🎉 Done');
    }

    this.currentSlide = state.current;

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
    this.elements.navigationElement.slideCurrent = (this.currentSlide !== null) ? this.currentSlide + 1 : null;
  }

  private updateConnectionStatus(status: string, className: ConnectionStatus): void {
    this.elements.toastElement.toast('info', status);
    this.elements.navigationElement.connectionStatus = className;
  }
}
