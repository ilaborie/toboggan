import { SlideId } from "./slides";

/**
 * Client identifier - UUID string
 */
export type ClientId = string;


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
