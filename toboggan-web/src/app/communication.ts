/**
 * Real-time Communication Service
 * Manages WebSocket connection life cycle, message handling, and latency monitoring
 */

import type { ClientId, Command, Notification, State } from "../types/index";
import { playChime } from "../utils/audio";
import { COMMANDS, DEFAULTS } from "../utils/constants";

export type CommunicationCallbacks = {
  onConnectionStatusChange: (status: ConnectionStatus) => void;
  onStateChange: (state: State) => void;
  onError: (error: string) => void;
};

export type Connecting = { status: "connecting" };
export type Connected = { status: "connected" };
export type Latency = { status: "latency"; latency: number };
export type Closed = { status: "closed" };
export type Reconnecting = {
  status: "reconnecting";
  attempt: number;
  maxAttempts: number;
  delaySeconds: number;
};
export type Error = { status: "error"; message: string };

/**
 * Connection status types
 */
export type ConnectionStatus = Connecting | Latency | Connected | Reconnecting | Closed | Error;

export const formatConnectionStatus = (status: ConnectionStatus): string => {
  switch (status.status) {
    case "connecting":
      return "📡 Connecting...";
    case "connected":
      return "🛜 Connected";
    case "latency":
      return `⏳ Ping latency ${status.latency}ms`;
    case "reconnecting":
      return `⛓️‍💥 Reconnecting in ${status.delaySeconds}s ${status.attempt}/${status.maxAttempts}`;
    case "closed":
      return "🚪 Closed";
    case "error":
      return `💥 Error: ${status.message}`;
  }
};

/**
 * WebSocket configuration
 */
export type WebSocketConfig = {
  readonly wsUrl: string;
  readonly maxRetries: number;
  readonly initialRetryDelay: number;
  readonly maxRetryDelay: number;
};

export class CommunicationService {
  private ws: WebSocket | null = null;
  private readonly config: WebSocketConfig;
  private readonly callbacks: CommunicationCallbacks;
  private readonly clientId: ClientId;
  private connectionRetryCount = 0;
  private retryDelay: number;
  private isDisposed = false;
  private pingInterval: number | null = null;

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
      this.callbacks.onConnectionStatusChange({ status: "connecting" });
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
      // console.log("📤 Sending command:", command);
      const message = JSON.stringify(command);
      this.ws.send(message);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      this.callbacks.onError(`Failed to send command: ${errorMessage}`);
    }
  }

  /**
   * Register this client with the server
   */
  public register(): void {
    this.sendCommand({ command: "Register", client: this.clientId });
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
    this.connectionRetryCount = 0;
    this.retryDelay = this.config.initialRetryDelay;
    this.startPinging();
    this.callbacks.onConnectionStatusChange({ status: "connected" });
  }

  private handleMessage(event: MessageEvent<string>): void {
    try {
      const notification: Notification = JSON.parse(event.data);
      // console.log("📥 Received notification:", notification);

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
        case "Blink":
          playChime();
          break;
      }
    } catch (error) {
      this.callbacks.onError(`Failed to parse server message: ${error}`);
    }
  }

  private handleClose(): void {
    console.log("WebSocket connection closed");
    this.stopPinging();
    this.callbacks.onConnectionStatusChange({ status: "closed" });
    if (!this.isDisposed) {
      this.scheduleReconnect();
    }
  }

  private handleError(): void {
    console.error("WebSocket error occurred");
    this.callbacks.onConnectionStatusChange({
      status: "error",
      message: "WebSocket error occurred",
    });
    this.callbacks.onError("Connection error occurred");
  }

  private scheduleReconnect(): void {
    if (this.connectionRetryCount >= this.config.maxRetries) {
      this.callbacks.onConnectionStatusChange({
        status: "error",
        message: `Max retries reached! (${this.config.maxRetries})`,
      });
      return;
    }

    this.connectionRetryCount++;
    const delaySeconds = this.retryDelay / 1000;

    this.callbacks.onConnectionStatusChange({
      status: "reconnecting",
      attempt: this.connectionRetryCount,
      maxAttempts: this.config.maxRetries,
      delaySeconds,
    });

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
  }

  /**
   * Send a ping command to measure latency
   */
  private sendPing(): void {
    console.time("ping-latency");
    this.sendCommand(COMMANDS.PING);
  }

  /**
   * Handle pong response and calculate latency
   */
  private handlePong(_serverTimestamp: string): void {
    console.timeEnd("ping-latency");
    this.callbacks.onConnectionStatusChange({ status: "latency", latency: 0 });
  }
}
