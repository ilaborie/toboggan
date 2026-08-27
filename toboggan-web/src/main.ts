import init, {
	start_app,
	start_mirror_app,
} from "../toboggan-wasm/pkg/toboggan_wasm";

import { appConfig, ensureTerminalFontLoaded, loadWasm } from "./boot";
import "./reset.css";
import "./main.css";
import "./state.css";

// A mirror is this very page, framed by the presenter view: same styles, same
// slide component, same viewport rules, so what the speaker watches is what the
// room watches rather than a second rendering that can drift from it. It opens
// no socket and runs no terminal, which is why it takes the early exit below —
// and why it needs neither `appConfig()` nor the terminal font.
//
// A query parameter rather than a page of its own: the three entry points would
// share one wasm chunk anyway, and `toboggan-server`'s build script asserts that
// every declared page exists in `dist/`, so a third one makes any server build
// against a stale `dist/` abort.
const mirrorPane = new URLSearchParams(location.search).get("mirror");

// Initialize the application when the DOM is loaded
document.addEventListener("DOMContentLoaded", async () => {
	// The terminal renders to a <canvas>, which silently falls back to a system
	// font if the web font isn't loaded yet. The faces are ~700 KB, and only
	// terminals need them, so the download is neither started nor awaited on the
	// way to the first slide — the deck used to wait on all of it before drawing
	// anything. Published as a memoised starter instead: whoever needs the font
	// first pays for it, everyone after that awaits the same promise.
	let fonts: Promise<void> | undefined;
	if (!mirrorPane) {
		window.tobogganFontsReady = () => {
			fonts ??= ensureTerminalFontLoaded();
			return fonts;
		};
	}

	if (!(await loadWasm(init))) {
		return;
	}

	const elt = document.querySelector("main");
	if (!elt) {
		console.error("🚨 Missing <main> element");
		return;
	}

	if (mirrorPane) {
		start_mirror_app(elt, mirrorPane);
		return;
	}

	start_app(appConfig(), elt);

	// With the deck up and the critical path clear, fetch the font anyway, so a
	// terminal opened later does not wait for it. Idle rather than immediate:
	// the point is to stay out of the way of the first slide.
	const warmFontCache = () => void window.tobogganFontsReady?.();
	if ("requestIdleCallback" in window) {
		requestIdleCallback(warmFontCache, { timeout: 5_000 });
	} else {
		setTimeout(warmFontCache, 2_000);
	}
});
