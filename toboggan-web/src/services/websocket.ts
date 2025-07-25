/**
 * WebSocket Connection Service
 * Manages WebSocket connection lifecycle and message handling
 */

import type { Command, Notification, ClientId } from '../types.js';

export interface WebSocketConfig {
  wsUrl: string;
  maxRetries: number;
  initialRetryDelay: number;
  maxRetryDelay: number;
}

export interface WebSocketCallbacks {
  onOpen: () => void;
  onNotification: (notification: Notification) => void;
  onClose: () => void;
  onError: (error: string) => void;
  onReconnecting: (attempt: number, maxAttempts: number, delaySeconds: number) => void;
  onMaxRetriesReached: () => void;
}

export class WebSocketService {
  private ws: WebSocket | null = null;
  private readonly config: WebSocketConfig;
  private readonly callbacks: WebSocketCallbacks;
  private readonly clientId: ClientId;
  private connectionRetryCount = 0;
  private retryDelay: number;
  private isDisposed = false;

  constructor(clientId: ClientId, config: WebSocketConfig, callbacks: WebSocketCallbacks) {
    this.clientId = clientId;
    this.config = config;
    this.callbacks = callbacks;
    this.retryDelay = config.initialRetryDelay;
  }

  /**
   * Connect to the WebSocket server
   */
  public connect(): void {
    if (this.isDisposed) return;
    
    try {
      this.ws = new WebSocket(this.config.wsUrl);
      
      this.ws.onopen = () => this.handleOpen();
      this.ws.onmessage = (event: MessageEvent<string>) => this.handleMessage(event);
      this.ws.onclose = () => this.handleClose();
      this.ws.onerror = () => this.handleError();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown connection error';
      this.callbacks.onError(`Connection failed: ${errorMessage}`);
      this.scheduleReconnect();
    }
  }

  /**
   * Send a command to the server
   */
  public sendCommand(command: Command): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      this.callbacks.onError('Not connected to server');
      return;
    }

    try {
      const message = JSON.stringify(command);
      console.log('Sending command:', command);
      this.ws.send(message);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      console.error('Failed to send command:', error);
      this.callbacks.onError(`Failed to send command: ${errorMessage}`);
    }
  }

  /**
   * Register this client with the server
   */
  public register(): void {
    this.sendCommand({
      command: 'Register',
      client: this.clientId,
      renderer: 'Html'
    });
  }

  /**
   * Dispose of the WebSocket connection
   */
  public dispose(): void {
    this.isDisposed = true;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  private handleOpen(): void {
    console.log('WebSocket connected');
    this.connectionRetryCount = 0;
    this.retryDelay = this.config.initialRetryDelay;
    this.callbacks.onOpen();
  }

  private handleMessage(event: MessageEvent<string>): void {
    try {
      const notification: Notification = JSON.parse(event.data);
      console.log('Received notification:', notification);
      this.callbacks.onNotification(notification);
    } catch (error) {
      console.error('Failed to parse notification:', error);
      this.callbacks.onError('Failed to parse server message');
    }
  }

  private handleClose(): void {
    console.log('WebSocket connection closed');
    this.callbacks.onClose();
    if (!this.isDisposed) {
      this.scheduleReconnect();
    }
  }

  private handleError(): void {
    console.error('WebSocket error occurred');
    this.callbacks.onError('Connection error occurred');
  }

  private scheduleReconnect(): void {
    if (this.connectionRetryCount >= this.config.maxRetries) {
      this.callbacks.onMaxRetriesReached();
      return;
    }

    this.connectionRetryCount++;
    const delaySeconds = this.retryDelay / 1000;
    
    this.callbacks.onReconnecting(this.connectionRetryCount, this.config.maxRetries, delaySeconds);

    setTimeout(() => {
      if (!this.isDisposed) {
        this.connect();
      }
    }, this.retryDelay);

    // Exponential backoff
    this.retryDelay = Math.min(this.retryDelay * 2, this.config.maxRetryDelay);
  }
}