import tobogganWasm, {
  AppConfig,
  start_app,
  WebSocketConfig,
} from "../toboggan-wasm/pkg/toboggan_wasm";

import "./reset.css";
import "./main.css";

// Initialize the application when the DOM is loaded
document.addEventListener("DOMContentLoaded", async () => {
  await tobogganWasm();
  const elt = document.querySelector("main");
  if (!elt) {
    console.error("🚨 Missing <main> element");
    return;
  }

  const config = new AppConfig();
  config.api_base_url = getEnvVar("VITE_API_BASE_URL", location.origin);

  const wsUrl = getEnvVar("VITE_WS_BASE_URL", `ws://${location.host}/api/ws`);
  config.websocket = new WebSocketConfig(wsUrl);
  config.websocket.max_retries = getEnvNumber("VITE_WS_MAX_RETRIES", 5);
  config.websocket.initial_retry_delay = getEnvNumber("VITE_WS_INITIAL_RETRY_DELAY", 1000);
  config.websocket.max_retry_delay = getEnvNumber("VITE_WS_MAX_RETRY_DELAY", 30000);

  start_app(config, elt);
});

/**
 * Get environment variable with fallback
 */
const getEnvVar = (key: keyof ImportMetaEnv, defaultValue: string): string =>
  import.meta.env[key] ?? defaultValue;

/**
 * Get environment variable as number with fallback
 */
const getEnvNumber = (key: keyof ImportMetaEnv, defaultValue: number): number => {
  const value = import.meta.env[key];
  const parsed = value ? parseInt(value, 10) : NaN;
  return Number.isNaN(parsed) ? defaultValue : parsed;
};
