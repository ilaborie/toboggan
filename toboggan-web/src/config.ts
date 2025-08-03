/**
 * Application Configuration Module
 * Manages environment variables with sensible defaults
 */

import type { WebSocketConfig } from "./app/communication";
import type { ClientId } from "./types";

export type AppConfig = {
  readonly clientId: ClientId;
  readonly apiBaseUrl: string;
  readonly websocket: WebSocketConfig;
};
/**
 * Get environment variable with fallback
 */
function getEnvVar(key: keyof ImportMetaEnv, defaultValue: string): string {
  return import.meta.env[key] ?? defaultValue;
}

/**
 * Get environment variable as number with fallback
 */
function getEnvNumber(key: keyof ImportMetaEnv, defaultValue: number): number {
  const value = import.meta.env[key];
  const parsed = value ? parseInt(value, 10) : NaN;
  return Number.isNaN(parsed) ? defaultValue : parsed;
}

/**
 * Create application configuration from environment variables
 */
export function createAppConfig(): AppConfig {
  const clientId = crypto.randomUUID();
  const apiBaseUrl = getEnvVar("VITE_API_BASE_URL", "http://localhost:8080");

  const wsBaseUrl = getEnvVar("VITE_WS_BASE_URL", "ws://localhost:8080");
  const websocket = {
    wsUrl: `${wsBaseUrl}/api/ws`,
    maxRetries: getEnvNumber("VITE_WS_MAX_RETRIES", 5),
    initialRetryDelay: getEnvNumber("VITE_WS_INITIAL_RETRY_DELAY", 1000),
    maxRetryDelay: getEnvNumber("VITE_WS_MAX_RETRY_DELAY", 30000),
  };

  return { clientId, apiBaseUrl, websocket };
}

/**
 * Default application configuration instance
 */
export const appConfig = createAppConfig();
