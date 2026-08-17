import init, { start_app } from "../toboggan-wasm/pkg/toboggan_wasm";

import { appConfig, ensureTerminalFontLoaded } from "./boot";
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

	start_app(appConfig(), elt);
});
