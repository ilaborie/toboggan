import type { Content } from "./contents";
import type { SlideId } from "./slides";
import type { Duration, Timestamp } from "./times";

export type Talk = {
  title: Content;
  date: string;
  titles: string[];
};

/**
 * Presentation state variants
 */
export type PausedState = {
  state: "Paused";
  current: SlideId;
  total_duration: Duration;
};

export type RunningState = {
  state: "Running";
  since: Timestamp;
  current: SlideId;
  total_duration: Duration;
};

export type DoneState = {
  state: "Done";
  current: SlideId;
  total_duration: Duration;
};

export type State = PausedState | RunningState | DoneState;
export type StateState = "Paused" | "Running" | "Done";
