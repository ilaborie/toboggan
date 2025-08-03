import type { SlideId } from "./slides";

/**
 * Client identifier - UUID string
 */
export type ClientId = string;

export type CommandHandler = {
  onCommand: (command: Command) => void;
};

/**
 * Commands that can be sent to the server
 */
export type FirstCommand = {
  command: "First";
};

export type LastCommand = {
  command: "Last";
};

export type NextCommand = {
  command: "Next";
};

export type PreviousCommand = {
  command: "Previous";
};

export type GoToCommand = {
  command: "GoTo";
  0: SlideId;
};

export type PauseCommand = {
  command: "Pause";
};

export type ResumeCommand = {
  command: "Resume";
};

export type RegisterCommand = {
  command: "Register";
  client: ClientId;
};

export type UnregisterCommand = {
  command: "Unregister";
  client: ClientId;
};

export type PingCommand = {
  command: "Ping";
};

export type BlinkCommand = {
  command: "Blink";
};

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
  | PingCommand
  | BlinkCommand;
