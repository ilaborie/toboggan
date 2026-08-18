import { AppConfig, WebSocketConfig } from "../toboggan-wasm/pkg/toboggan_wasm";

/**
 * Builds the configuration both entry points hand to wasm.
 *
 * Shared because the deck and the presenter view are the same application
 * against the same server — the only thing that ever differed between the two
 * copies of this was which `start_*` it called, and a config that drifts
 * between two pages of one app is a bug waiting for a talk to be given.
 */
export const appConfig = (): AppConfig => {
	const config = new AppConfig();
	config.api_base_url = getEnvVar("VITE_API_BASE_URL", location.origin);

	const wsUrl = getEnvVar("VITE_WS_BASE_URL", defaultWsUrl());
	config.websocket = new WebSocketConfig(wsUrl);
	config.websocket.max_retries = getEnvNumber("VITE_WS_MAX_RETRIES", 5);
	config.websocket.initial_retry_delay = getEnvNumber(
		"VITE_WS_INITIAL_RETRY_DELAY",
		1000,
	);
	config.websocket.max_retry_delay = getEnvNumber(
		"VITE_WS_MAX_RETRY_DELAY",
		30000,
	);
	return config;
};

/**
 * Preload the bundled terminal Nerd Font (all four faces the renderer uses) so
 * the canvas renderer measures cell width and draws glyphs with the right font.
 */
export const ensureTerminalFontLoaded = async (): Promise<void> => {
	if (!("fonts" in document)) {
		return;
	}
	const family = '"JetBrainsMono Nerd Font Mono"';
	const faces = [
		`16px ${family}`,
		`bold 16px ${family}`,
		`italic 16px ${family}`,
		`bold italic 16px ${family}`,
	];
	await Promise.allSettled(faces.map((face) => document.fonts.load(face)));
};

/**
 * The server this page came from, as a WebSocket URL.
 *
 * Matching the page's scheme is not cosmetic: a browser refuses a plain `ws://`
 * socket opened from an `https://` page, so a deck served over TLS would never
 * connect.
 */
const defaultWsUrl = (): string => {
	const scheme = location.protocol === "https:" ? "wss:" : "ws:";
	return `${scheme}//${location.host}/api/ws`;
};

/**
 * Get environment variable with fallback.
 *
 * The server URLs are set only in `.env.development`, so in a production build
 * these fall through to the location-derived defaults and the deck works on
 * whatever port and host it is actually served from.
 */
const getEnvVar = (key: keyof ImportMetaEnv, defaultValue: string): string =>
	import.meta.env[key] ?? defaultValue;

/**
 * Get environment variable as number with fallback
 */
const getEnvNumber = (
	key: keyof ImportMetaEnv,
	defaultValue: number,
): number => {
	const value = import.meta.env[key];
	const parsed = value ? parseInt(value, 10) : NaN;
	return Number.isNaN(parsed) ? defaultValue : parsed;
};
