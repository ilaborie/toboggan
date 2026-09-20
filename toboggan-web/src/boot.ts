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
 * Preload the bundled terminal Nerd Font (the four text faces the renderer uses
 * and the icon face they fall back into) so the canvas renderer measures cell
 * width and draws glyphs with the right font.
 */
export const ensureTerminalFontLoaded = async (): Promise<void> => {
	if (!("fonts" in document)) {
		return;
	}
	const text = '"JetBrainsMono Nerd Font Mono"';
	// Probed with an icon rather than the default `BESbswy`: this face carries
	// the private-use planes and nothing else, so a Latin probe would match no
	// codepoint in it the day it grows a `unicode-range`.
	const symbols = '"JetBrainsMono Nerd Font Symbols"';
	const faces = [
		{ font: `16px ${text}` },
		{ font: `bold 16px ${text}` },
		{ font: `italic 16px ${text}` },
		{ font: `bold italic 16px ${text}` },
		{ font: `16px ${symbols}`, probe: "\u{f0035}" },
	];
	const results = await Promise.allSettled(
		faces.map(({ font, probe }) => document.fonts.load(font, probe)),
	);
	// `allSettled` is chosen so a missing face does not stop the page, and the
	// array was then thrown away — which made the fallback this function exists
	// to avoid completely invisible. A terminal measured against the wrong font
	// draws its box-drawing and powerline glyphs out of line.
	//
	// A resolved-but-empty result counts as a failure too: `load()` matched no
	// face, which is what a family named here and not in `main.css` looks like.
	const failed = results.filter(
		(result) => result.status === "rejected" || result.value.length === 0,
	);
	if (failed.length > 0) {
		console.error(
			`⚠️ ${failed.length}/${faces.length} terminal font faces failed to load; ` +
				"terminals will fall back to a system font and may render misaligned",
			failed.map((result) =>
				result.status === "rejected" ? result.reason : "matched no @font-face",
			),
		);
	}
};

/**
 * Loads the wasm module, reporting a failure into the page rather than the void.
 *
 * `await init()` was unguarded, next to a careful null-check on `<main>` — and a
 * rejection here is far more likely: a 404 on the `.wasm` from a stale `dist/`
 * or a rewritten path, a CSP without `wasm-unsafe-eval`, a MIME type that is not
 * `application/wasm`, a truncated download. Every one of those became an
 * unhandled promise rejection and a white screen, with the answer in devtools —
 * which is not where someone stands two minutes before a talk.
 *
 * Returns whether the caller should carry on.
 */
export const loadWasm = async (
	init: () => Promise<unknown>,
): Promise<boolean> => {
	try {
		await init();
		return true;
	} catch (error) {
		console.error("🚨 Failed to load the Toboggan wasm module", error);
		const message = document.createElement("p");
		message.setAttribute("role", "alert");
		message.style.cssText =
			"margin:2rem;font:16px/1.5 system-ui,sans-serif;color:#e6edf5";
		message.textContent =
			"Could not load the presentation engine. Check that the server is " +
			"serving toboggan_wasm_bg.wasm, then reload.";
		document.body.replaceChildren(message);
		return false;
	}
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
	if (value && Number.isNaN(parsed)) {
		// Silently substituting the default made `VITE_WS_MAX_RETRIES=five` look
		// like it had been applied.
		console.warn(
			`⚠️ ${key}="${value}" is not a number; using ${defaultValue} instead`,
		);
	}
	return Number.isNaN(parsed) ? defaultValue : parsed;
};
