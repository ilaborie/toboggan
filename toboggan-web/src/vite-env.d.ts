/// <reference types="vite/client" />

interface ImportMetaEnv {
	readonly VITE_WS_BASE_URL?: string;
	readonly VITE_API_BASE_URL?: string;
	readonly VITE_WS_MAX_RETRIES?: string;
	readonly VITE_WS_INITIAL_RETRY_DELAY?: string;
	readonly VITE_WS_MAX_RETRY_DELAY?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}

interface Window {
	/**
	 * Starts (once) and returns the load of the bundled terminal Nerd Font faces.
	 *
	 * Published so the wasm terminal can pull in exactly the fonts it measures
	 * against, at the moment it needs them, instead of the deck blocking its own
	 * first render on ~700 KB nothing on screen is using yet.
	 */
	tobogganFontsReady?: () => Promise<void>;
}
