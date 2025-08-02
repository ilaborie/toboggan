/**
 * Real-time Communication Service
 * Manages WebSocket connection lifecycle, message handling, and latency monitoring
 */

import { COMMANDS, DEFAULTS } from "../constants.js";
import type { ClientId, Command, ConnectionStatus, Notification, State, WebSocketConfig } from "../types.js";

export interface CommunicationCallbacks {
  onConnectionStatusChange: (status: ConnectionStatus) => void;
  onStateChange: (state: State) => void;
  onError: (error: string) => void;

  onReconnecting: (attempt: number, maxAttempts: number, delaySeconds: number) => void;
  onMaxRetriesReached: () => void;
  onLatencyUpdate: (latency: number) => void;
}

/**
 * Connection status types
 */
export type ConnectionStatus =
  | "connecting"
  | "connected"
  | "disconnected"
  | "reconnecting"
  | "running"
  | "paused"
  | "done";


/**
 * WebSocket configuration
 */
export interface WebSocketConfig {
  readonly wsUrl: string;
  readonly maxRetries: number;
  readonly initialRetryDelay: number;
  readonly maxRetryDelay: number;
}

export class CommunicationService {
  private ws: WebSocket | null = null;
  private readonly config: WebSocketConfig;
  private readonly callbacks: CommunicationCallbacks;
  private readonly clientId: ClientId;
  private connectionRetryCount = 0;
  private retryDelay: number;
  private isDisposed = false;
  private pingInterval: number | null = null;
  private pendingPings = new Map<number, number>(); // timestamp -> sent time
  // biome-ignore lint/correctness/noUnusedPrivateClassMembers: Used in sendPing method for ping ID generation
  private pingCounter = 0;

  constructor(clientId: ClientId, config: WebSocketConfig, callbacks: CommunicationCallbacks) {
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
      const errorMessage = error instanceof Error ? error.message : "Unknown connection error";
      this.callbacks.onError(`Connection failed: ${errorMessage}`);
      this.scheduleReconnect();
    }
  }

  /**
   * Send a command to the server
   */
  public sendCommand(command: Command): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      this.callbacks.onError("Not connected to server");
      return;
    }

    try {
      const message = JSON.stringify(command);
      console.log("Sending command:", command);
      this.ws.send(message);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      console.error("Failed to send command:", error);
      this.callbacks.onError(`Failed to send command: ${errorMessage}`);
    }
  }

  /**
   * Register this client with the server
   */
  public register(): void {
    this.sendCommand({
      command: "Register",
      client: this.clientId,
      renderer: "Html",
    });
  }

  /**
   * Dispose of the WebSocket connection
   */
  public dispose(): void {
    this.isDisposed = true;
    this.stopPinging();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  private handleOpen(): void {
    console.log("WebSocket connected");
    this.connectionRetryCount = 0;
    this.retryDelay = this.config.initialRetryDelay;
    this.startPinging();
    this.callbacks.onConnectionStatusChange("connected");
  }

  private handleMessage(event: MessageEvent<string>): void {
    try {
      const notification: Notification = JSON.parse(event.data);
      console.log("Received notification:", notification);

      switch (notification.type) {
        case "State":
          this.callbacks.onStateChange(notification.state);
          break;
        case "Error":
          this.callbacks.onError(notification.message);
          break;
        case "Pong":
          this.handlePong(notification.timestamp);
          break;
      }

    } catch (error) {
      console.error("Failed to parse notification:", error);
      this.callbacks.onError("Failed to parse server message");
    }
  }

  private handleClose(): void {
    console.log("WebSocket connection closed");
    this.stopPinging();
    this.callbacks.onConnectionStatusChange("done");
    if (!this.isDisposed) {
      this.scheduleReconnect();
    }
  }

  private handleError(): void {
    console.error("WebSocket error occurred");
    this.callbacks.onError("Connection error occurred");
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

  /**
   * Start periodic ping to monitor connection latency
   */
  private startPinging(): void {
    this.stopPinging(); // Clear any existing interval

    this.pingInterval = window.setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.sendPing();
      }
    }, DEFAULTS.PING_INTERVAL);
  }

  /**
   * Stop periodic pinging
   */
  private stopPinging(): void {
    if (this.pingInterval !== null) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
    this.pendingPings.clear();
  }

  /**
   * Send a ping command to measure latency
   */
  private sendPing(): void {
    const pingId = ++this.pingCounter;
    const sentTime = Date.now();

    this.pendingPings.set(pingId, sentTime);
    this.sendCommand(COMMANDS.PING);

    // Clean up old pings after timeout
    setTimeout(() => {
      this.pendingPings.delete(pingId);
    }, 10000); // 10 seconds timeout
  }

  /**
   * Handle pong response and calculate latency
   */
  private handlePong(_serverTimestamp: string): void {
    const receivedTime = Date.now();

    // Find the most recent pending ping
    let latestPing: [number, number] | undefined;
    for (const [pingId, sentTime] of this.pendingPings.entries()) {
      if (!latestPing || sentTime > latestPing[1]) {
        latestPing = [pingId, sentTime];
      }
    }

    if (latestPing) {
      const [pingId, sentTime] = latestPing;
      const latency = receivedTime - sentTime;

      this.pendingPings.delete(pingId);
      this.callbacks.onLatencyUpdate(latency);

      console.log(`Ping latency: ${latency}ms`);
    }
  }
}
