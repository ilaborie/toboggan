import type { State } from "./talks";
import type { Timestamp } from "./times";

/**
 * Notifications sent by the server
 */
export type StateNotification = {
  type: "State";
  timestamp: Timestamp;
  state: State;
};

export type ErrorNotification = {
  type: "Error";
  timestamp: Timestamp;
  message: string;
};

export type PongNotification = {
  type: "Pong";
  timestamp: Timestamp;
};

export type BlinkNotification = {
  type: "Blink";
};

export type Notification =
  | StateNotification
  | ErrorNotification
  | PongNotification
  | BlinkNotification;
