import { State } from "./talks";
import { Timestamp } from "./times";

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


export type Notification = StateNotification | ErrorNotification | PongNotification;
