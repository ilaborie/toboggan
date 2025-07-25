/**
 * Presentation Controller
 * Coordinates the presentation state and services
 */

import type { 
  AppConfig, 
  Command, 
  ConnectionStatus, 
  Notification, 
  SlideId, 
  State 
} from '../types.js';
import { WebSocketService, type WebSocketCallbacks } from '../services/websocket.js';
import { SlidesApiService } from '../services/slidesApi.js';
import { SlideRenderer } from '../services/slideRenderer.js';
import { TimerService } from '../services/timer.js';
import { showError } from '../utils/dom.js';

export interface PresentationElements {
  connectionStatus: HTMLSpanElement;
  slideCounter: HTMLSpanElement;
  durationDisplay: HTMLSpanElement;
  errorDisplay: HTMLDivElement;
  appElement: HTMLDivElement;
}

export class PresentationController {
  private readonly elements: PresentationElements;
  private readonly websocket: WebSocketService;
  private readonly slidesApi: SlidesApiService;
  private readonly slideRenderer: SlideRenderer;
  private readonly timer: TimerService;
  
  private currentSlide: SlideId | null = null;

  constructor(
    clientId: string,
    config: AppConfig,
    elements: PresentationElements
  ) {
    this.elements = elements;
    
    // Initialize services
    this.slidesApi = new SlidesApiService(config.apiBaseUrl);
    this.slideRenderer = new SlideRenderer(elements.appElement);
    this.timer = new TimerService(elements.durationDisplay);
    
    // Initialize WebSocket with callbacks
    const wsCallbacks: WebSocketCallbacks = {
      onOpen: () => this.handleWebSocketOpen(),
      onNotification: (notification) => this.handleNotification(notification),
      onClose: () => this.handleWebSocketClose(),
      onError: (error) => this.handleError(error),
      onReconnecting: (attempt, max, delay) => this.handleReconnecting(attempt, max, delay),
      onMaxRetriesReached: () => this.handleMaxRetriesReached()
    };
    
    this.websocket = new WebSocketService(
      clientId,
      {
        wsUrl: config.wsUrl,
        maxRetries: config.maxRetries,
        initialRetryDelay: config.initialRetryDelay,
        maxRetryDelay: config.maxRetryDelay
      },
      wsCallbacks
    );
  }

  /**
   * Start the presentation controller
   */
  public start(): void {
    this.updateConnectionStatus('Connecting...', 'connecting');
    this.websocket.connect();
  }

  /**
   * Send a navigation command
   */
  public sendCommand(command: Command): void {
    this.websocket.sendCommand(command);
  }

  /**
   * Dispose of all resources
   */
  public dispose(): void {
    this.timer.stop();
    this.websocket.dispose();
  }

  private handleWebSocketOpen(): void {
    this.updateConnectionStatus('Connected', 'connected');
    this.websocket.register();
  }

  private handleWebSocketClose(): void {
    this.updateConnectionStatus('Disconnected', 'disconnected');
    this.slidesApi.clearCache();
    this.timer.stop();
  }

  private handleReconnecting(attempt: number, max: number, delaySeconds: number): void {
    this.updateConnectionStatus(
      `Reconnecting in ${delaySeconds}s... (${attempt}/${max})`,
      'reconnecting'
    );
  }

  private handleMaxRetriesReached(): void {
    this.handleError('Max reconnection attempts reached. Please refresh the page.');
  }

  private handleError(message: string): void {
    showError(this.elements.errorDisplay, message);
  }

  private handleNotification(notification: Notification): void {
    switch (notification.type) {
      case 'State':
        this.handleStateNotification(notification.state);
        break;
      case 'Error':
        this.handleError(notification.message);
        break;
      case 'Pong':
        console.log('Received pong');
        break;
      default:
        console.warn('Unknown notification type:', (notification as any).type);
    }
  }

  private async handleStateNotification(state: State): Promise<void> {
    // Update timer state
    this.timer.updateState(state);
    
    // Update current slide
    if ('Running' in state) {
      this.currentSlide = state.Running.current;
      this.updateConnectionStatus('Running', 'running');
    } else if ('Paused' in state) {
      this.currentSlide = state.Paused.current;
      this.updateConnectionStatus('Paused', 'paused');
    } else if ('Done' in state) {
      this.currentSlide = state.Done.current;
      this.updateConnectionStatus('Done', 'done');
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
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      console.error('Failed to load slide:', error);
      this.handleError(`Failed to load slide: ${errorMessage}`);
    }
  }

  private updateSlideCounter(): void {
    if (this.currentSlide !== null) {
      const displayNumber = this.slidesApi.getSlideDisplayNumber(this.currentSlide);
      const total = this.slidesApi.getTotalSlides();
      const totalStr = total ? ` / ${total}` : '';
      this.elements.slideCounter.textContent = `Slide: ${displayNumber}${totalStr}`;
    } else {
      this.elements.slideCounter.textContent = 'Slide: - / -';
    }
  }

  private updateConnectionStatus(status: string, className: ConnectionStatus): void {
    this.elements.connectionStatus.textContent = status;
    this.elements.connectionStatus.className = className;
  }
}