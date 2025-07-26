// TypeScript interfaces for Toboggan WebSocket protocol

/**
 * Slide identifier - u8 value starting from 0
 */
export type SlideId = number;

/**
 * Client identifier - UUID string
 */
export type ClientId = string;

/**
 * ISO 8601 timestamp string
 */
export type Timestamp = string;

/**
 * Renderer types for client registration
 */
export type Renderer = "Title" | "Thumbnail" | "Html" | "Raw";

/**
 * Content types for slide content
 */
export interface EmptyContent {
  type: "Empty";
}

export interface TextContent {
  type: "Text";
  text: string;
}

export interface HtmlContent {
  type: "Html";
  raw: string;
  alt: string;
}

export interface MdContent {
  type: "Md";
  content: string;
  alt?: string;
}

export interface IFrameContent {
  type: "IFrame";
  url: string;
  alt?: string;
}

export type Content = EmptyContent | TextContent | HtmlContent | MdContent | IFrameContent;

/**
 * Slide structure
 */
export interface Slide {
  kind: "Cover" | "Part" | "Standard";
  style: string[];
  title: Content;
  body: Content;
  notes: Content;
}

/**
 * Presentation state variants
 */
export interface PausedState {
  state: "Paused";
  current: SlideId;
  total_duration: Duration;
}

export interface RunningState {
  state: "Running";
  since: Timestamp;
  current: SlideId;
  total_duration: Duration;
}

export interface DoneState {
  state: "Done";
  current: SlideId;
  total_duration: Duration;
}

export type State = PausedState | RunningState | DoneState;

/**
 * Duration structure
 */
export interface Duration {
  secs: number;
  // XXX We don't care about nanos
  // nanos: number;
}

/**
 * Commands that can be sent to the server
 */
export interface FirstCommand {
  command: "First";
}

export interface LastCommand {
  command: "Last";
}

export interface NextCommand {
  command: "Next";
}

export interface PreviousCommand {
  command: "Previous";
}

export interface GoToCommand {
  command: "GoTo";
  0: SlideId;
}

export interface PauseCommand {
  command: "Pause";
}

export interface ResumeCommand {
  command: "Resume";
}

export interface RegisterCommand {
  command: "Register";
  client: ClientId;
  renderer: Renderer;
}

export interface UnregisterCommand {
  command: "Unregister";
  client: ClientId;
}

export interface PingCommand {
  command: "Ping";
}

export type Command =
  | FirstCommand
  | LastCommand
  | NextCommand
  | PreviousCommand
  | GoToCommand
  | PauseCommand
  | ResumeCommand
  | RegisterCommand
  | UnregisterCommand
  | PingCommand;

/**
 * Notifications sent by the server
 */
export interface StateNotification {
  type: "State";
  timestamp: Timestamp;
  state: State;
}

export interface ErrorNotification {
  type: "Error";
  timestamp: Timestamp;
  message: string;
}

export interface PongNotification {
  type: "Pong";
  timestamp: Timestamp;
}

export type Notification = StateNotification | ErrorNotification | PongNotification;

/**
 * API response structures
 */
export interface TalkResponse {
  talk: {
    title: Content;
    date: string;
    slides: Slide[];
  };
}

export interface SlidesResponse {
  slides: Record<string, Slide>;
}

export interface HealthResponse {
  status: "OK";
  started_at: Timestamp;
  elapsed: string;
  talk: string;
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
 * DOM element getter type safety
 */
export type RequiredElement<T extends Element> = T;

/**
 * WebSocket configuration
 */
export interface WebSocketConfig {
  readonly wsUrl: string;
  readonly maxRetries: number;
  readonly initialRetryDelay: number;
  readonly maxRetryDelay: number;
}

/**
 * Application configuration
 */
export interface AppConfig {
  readonly apiBaseUrl: string;
  readonly websocket: WebSocketConfig;
}

/**
 * Slides data cache
 */
export interface SlidesCache {
  slides: Record<string, Slide>;
  orderedIds: SlideId[];
}
