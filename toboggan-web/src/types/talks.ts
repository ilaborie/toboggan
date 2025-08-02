import { SlideId } from "./slides";
import { Duration, Timestamp } from "./times";

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
export type StateState = 'Paused' | 'Running' | 'Done';
