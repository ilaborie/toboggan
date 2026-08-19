// Keyboard navigation for a deck exported with `toboggan build -o deck.html`.
//
// The export is every slide of the deck in one file. Without this it is a page
// you scroll, with every reveal step already on screen — which is also what
// `action.yml` publishes to GitHub Pages. With it, the file is a presentation:
// one slide at a time, steps revealed as you go, driven by the same keys as
// every other Toboggan client.
//
// Everything it does is gated on the `toboggan-js` class it adds to <html>, and
// the matching CSS is `@media screen` only. So a file opened with scripting off,
// or sent to a printer, is still the whole deck in order — which is what the PDF
// export depends on.
(() => {
	const slides = [...document.querySelectorAll(".toboggan-slide")];
	if (slides.length === 0) {
		return;
	}
	document.documentElement.classList.add("toboggan-js");

	let slideIndex = 0;
	// How many steps of the current slide have been revealed, matching the web
	// client: 0 is none, 1 is the first one showing.
	let revealed = 0;

	const stepsOf = (index) => [...slides[index].querySelectorAll(".step")];

	function render() {
		slides.forEach((slide, index) =>
			slide.classList.toggle("current", index === slideIndex),
		);
		stepsOf(slideIndex).forEach((step, index) => {
			step.classList.toggle("step-done", index < revealed);
			step.classList.toggle("step-current", index + 1 === revealed);
		});
		// replaceState rather than assigning location.hash: the URL should always
		// name the slide on screen, so it can be copied or bookmarked mid-talk —
		// but not by turning every keystroke into a history entry to back out of.
		history.replaceState(null, "", `#slide-${slideIndex + 1}`);
	}

	function goToSlide(index, atLastStep) {
		slideIndex = Math.max(0, Math.min(index, slides.length - 1));
		revealed = atLastStep ? stepsOf(slideIndex).length : 0;
		render();
	}

	function nextStep() {
		if (revealed < stepsOf(slideIndex).length) {
			revealed += 1;
			render();
		} else if (slideIndex < slides.length - 1) {
			goToSlide(slideIndex + 1, false);
		}
	}

	function previousStep() {
		if (revealed > 0) {
			revealed -= 1;
			render();
		} else if (slideIndex > 0) {
			// Stepping back into a slide lands on its last step, not its first:
			// the presenter is retracing what the room has already seen.
			goToSlide(slideIndex - 1, true);
		}
	}

	function toggleFullscreen() {
		if (document.fullscreenElement) {
			document.exitFullscreen();
		} else {
			document.documentElement.requestFullscreen().catch((error) => {
				// An empty catch here left the presenter pressing `f` in front of
				// the room with no way to tell "the key is not bound" from "the
				// browser said no" — which it does for a missing user gesture, a
				// `fullscreen` permissions-policy in an iframe, or Safari on a
				// non-video element.
				console.error("Toboggan: the browser refused fullscreen", error);
			});
		}
	}

	const actions = {
		ArrowRight: () => goToSlide(slideIndex + 1, false),
		ArrowLeft: () => goToSlide(slideIndex - 1, false),
		ArrowDown: nextStep,
		ArrowUp: previousStep,
		" ": nextStep,
		// What a presenter remote sends. Bound to the steps, so the remote walks
		// the whole deck instead of skipping every reveal on the way.
		PageDown: nextStep,
		PageUp: previousStep,
		Backspace: previousStep,
		Home: () => goToSlide(0, false),
		End: () => goToSlide(slides.length - 1, false),
		f: toggleFullscreen,
		F: toggleFullscreen,
	};

	addEventListener("keydown", (event) => {
		// A modified key belongs to the browser, not the deck. Without this,
		// Cmd+F and Ctrl+F toggled fullscreen and swallowed find-in-page — the
		// main way anyone navigates a single-file export.
		if (event.ctrlKey || event.metaKey || event.altKey) {
			return;
		}
		const action = actions[event.key];
		if (!action) {
			return;
		}
		// Every one of these keys already means something to the browser: space
		// and the arrows scroll, PageUp/PageDown page, Backspace used to go back.
		event.preventDefault();
		action();
	});

	function slideFromHash() {
		const number = Number.parseInt(location.hash.replace("#slide-", ""), 10);
		return Number.isNaN(number) ? 0 : number - 1;
	}

	// Following a link to `#slide-12`, or editing the URL by hand, is a jump.
	// `render` writes the hash with replaceState, which deliberately does not
	// fire this — so there is no loop to break.
	addEventListener("hashchange", () => goToSlide(slideFromHash(), false));

	goToSlide(slideFromHash(), false);
})();
