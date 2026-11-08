// Navigation (keys and touch) and auto-fit for a deck exported with
// `toboggan build -o deck.html`.
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

	// Whether anything in the article reaches past the bottom or right edge of
	// `slide`. Only the article: it is all the zoom shrinks, so a title too wide
	// on its own would drive the zoom to its floor and still not fit. Every
	// descendant is measured, not just the article box: a code block clips its
	// own lines, but the spans inside it still report where they would end.
	function overflows(slide, section) {
		const frame = slide.getBoundingClientRect();
		const drawn = section.querySelectorAll(
			":scope > article, :scope > article *",
		);
		return [...drawn].some((element) => {
			const box = element.getBoundingClientRect();
			return (
				box.height > 0 &&
				(box.bottom > frame.bottom + 0.5 || box.right > frame.right + 0.5)
			);
		});
	}

	// Shrinks a `fit` slide to the largest zoom at which it fits the screen,
	// never below `FIT_FLOOR`: past that the text is unreadable anyway, so the
	// slide stays clipped and the console says which one.
	// Hidden steps are laid out too (they are only transparent), so the zoom
	// does not depend on which steps are revealed, and revealing one never
	// changes it.
	// A bisection rather than one ratio: shrinking also gives lines more width,
	// so fewer wrap, and the height does not scale linearly with the zoom.
	const FIT_FLOOR = 0.25;
	function fit(slide) {
		const section = slide.querySelector(":scope > section.fit");
		if (!section) {
			return;
		}
		const setZoom = (zoom) => section.style.setProperty("--toboggan-fit", zoom);
		setZoom(1);
		if (!overflows(slide, section)) {
			return;
		}
		let fits = FIT_FLOOR;
		let tooBig = 1;
		for (let round = 0; round < 8; round += 1) {
			const zoom = (fits + tooBig) / 2;
			setZoom(zoom);
			if (overflows(slide, section)) {
				tooBig = zoom;
			} else {
				fits = zoom;
			}
		}
		setZoom(fits);
		if (fits === FIT_FLOOR && overflows(slide, section)) {
			console.warn(
				`Toboggan: slide ${slides.indexOf(slide) + 1} still overflows at the minimum fit zoom`,
			);
		}
	}

	// Fit again whenever what was measured may have changed size: the window,
	// a web font, an image. A font is fetched only once a slide that uses it is
	// shown, so it arrives after that slide's first fit, measured with a fallback
	// of other metrics; an image without set dimensions is measured at height 0
	// until it loads. An image's `load` does not bubble, hence the capture.
	const refit = () => fit(slides[slideIndex]);
	addEventListener("resize", refit);
	document.fonts?.addEventListener("loadingdone", refit);
	document.addEventListener(
		"load",
		(event) => {
			if (
				event.target instanceof HTMLImageElement &&
				slides[slideIndex].contains(event.target)
			) {
				refit();
			}
		},
		true,
	);

	function render() {
		slides.forEach((slide, index) =>
			slide.classList.toggle("current", index === slideIndex),
		);
		fit(slides[slideIndex]);
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

	// Touch: a phone or tablet has none of the keys above. A swipe left or a
	// tap on the right two thirds moves forward; a swipe right or a tap on the
	// left third moves back. Both walk the steps, like a presenter remote.
	// Touch events rather than pointer events: the browser cancels a pointer
	// as soon as it starts to pan, and the swipe would never finish.
	const SWIPE_MIN_PX = 50;
	const TAP_MAX_PX = 10;
	const TAP_MAX_MS = 500;
	let touchStart = null;

	// What a touch has to leave alone: a link or a control does its own job.
	const ownsTheTouch = (target) =>
		target.closest("a, button, input, select, textarea, label, summary") !==
		null;

	// A block that scrolls sideways (a long code line) needs a swipe to scroll
	// its content. A tap on it still navigates: a tap scrolls nothing.
	function scrollsSideways(target) {
		for (
			let element = target;
			element && element !== document.body;
			element = element.parentElement
		) {
			const overflowX = getComputedStyle(element).overflowX;
			if (
				(overflowX === "auto" || overflowX === "scroll") &&
				element.scrollWidth > element.clientWidth
			) {
				return true;
			}
		}
		return false;
	}

	addEventListener(
		"touchstart",
		(event) => {
			// A second finger is a pinch zoom, and once the page is zoomed in a
			// one-finger drag pans it: neither is a navigation.
			const zoomedIn = (window.visualViewport?.scale ?? 1) > 1.01;
			if (
				zoomedIn ||
				event.touches.length !== 1 ||
				!(event.target instanceof Element)
			) {
				touchStart = null;
				return;
			}
			const touch = event.touches[0];
			touchStart = {
				x: touch.clientX,
				y: touch.clientY,
				at: Date.now(),
				target: event.target,
			};
		},
		{ passive: true },
	);

	addEventListener("touchcancel", () => {
		touchStart = null;
	});

	addEventListener("touchend", (event) => {
		const start = touchStart;
		touchStart = null;
		if (!start || event.touches.length > 0 || ownsTheTouch(start.target)) {
			return;
		}
		const touch = event.changedTouches[0];
		const dx = touch.clientX - start.x;
		const dy = touch.clientY - start.y;

		// Mostly horizontal, and long enough not to be a shaky tap.
		const isSwipe =
			Math.abs(dx) >= SWIPE_MIN_PX && Math.abs(dx) > 2 * Math.abs(dy);
		const isTap =
			Math.abs(dx) <= TAP_MAX_PX &&
			Math.abs(dy) <= TAP_MAX_PX &&
			Date.now() - start.at <= TAP_MAX_MS;

		if (isSwipe && !scrollsSideways(start.target)) {
			if (dx < 0) {
				nextStep();
			} else {
				previousStep();
			}
		} else if (isTap) {
			// Cancels the click the browser would fire next: it would land on
			// whatever sits at that spot on the new slide, a link included.
			event.preventDefault();
			if (touch.clientX < innerWidth / 3) {
				previousStep();
			} else {
				nextStep();
			}
		}
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
