import init, { start_presenter_app } from "../toboggan-wasm/pkg/toboggan_wasm";

import { appConfig, loadWasm } from "./boot";
import "./reset.css";
import "./presenter.css";

// The presenter view previews slides but never runs their terminals, so it does
// not preload the terminal font — and `state.css` styles the full-screen deck,
// which this page is not.
//
// `main.css` is not loaded either, for the stronger version of the same reason:
// it is the deck's stylesheet, and the deck is now rendered by the two mirror
// iframes, each of which loads it in a document of its own. Loading it here as
// well is what let a deck's `_head.html` — `main { background: … }` in the
// packaged guide — repaint the speaker's chrome with the projector's backdrop.
document.addEventListener("DOMContentLoaded", async () => {
	if (!(await loadWasm(init))) {
		return;
	}

	const elt = document.querySelector("main");
	if (!elt) {
		console.error("🚨 Missing <main> element");
		return;
	}

	start_presenter_app(appConfig(), elt);
});
