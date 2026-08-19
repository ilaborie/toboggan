import init, { start_presenter_app } from "../toboggan-wasm/pkg/toboggan_wasm";

import { appConfig, loadWasm } from "./boot";
import "./reset.css";
import "./main.css";

// The presenter view previews slides but never runs their terminals, so it does
// not preload the terminal font — and `state.css` styles the full-screen deck,
// which this page is not.
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
