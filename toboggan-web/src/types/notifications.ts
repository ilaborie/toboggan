import { State } from "./talks";
import { Timestamp } from "./times";

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
