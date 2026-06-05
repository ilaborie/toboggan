import init, {
	AppConfig,
	start_app,
	WebSocketConfig,
} from "../toboggan-wasm/pkg/toboggan_wasm";

import "./reset.css";
import "./main.css";
import "./state.css";

// Initialize the application when the DOM is loaded
document.addEventListener("DOMContentLoaded", async () => {
	await init();

	// The terminal renders to a <canvas>, which silently falls back to a system
	// font if the web font isn't loaded yet. Fetch the faces up front so the
	// first render measures and draws with the bundled Nerd Font.
	await ensureTerminalFontLoaded();

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
	config.websocket.initial_retry_delay = getEnvNumber(
		"VITE_WS_INITIAL_RETRY_DELAY",
		1000,
	);
	config.websocket.max_retry_delay = getEnvNumber(
		"VITE_WS_MAX_RETRY_DELAY",
		30000,
	);

	start_app(config, elt);
});

/**
 * Preload the bundled terminal Nerd Font (all four faces the renderer uses) so
 * the canvas renderer measures cell width and draws glyphs with the right font.
 */
const ensureTerminalFontLoaded = async (): Promise<void> => {
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
 * Get environment variable with fallback
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
