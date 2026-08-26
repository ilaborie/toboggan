import {
	type APIRequestContext,
	expect,
	type Locator,
	type Page,
	test,
} from "@playwright/test";

/**
 * Who owns the keyboard, and what maximizing does to the window.
 *
 * Both are things only a browser can answer. The deck's key handler sits on
 * `window` and every keystroke typed at a terminal's shell reaches it — rioterm
 * calls `preventDefault` but never `stopPropagation` — so the only thing keeping
 * `space` out of the presentation is the terminal's claim on the keyboard. And
 * the claim used to be a focus test, which is invisible from Rust and was wrong
 * for every click that did not land on the canvas itself.
 */
test.describe.configure({ mode: "serial" });

/** How long "nothing happened" is given to happen anyway. */
const SETTLE_MS = 400;

/**
 * The two terminals, kept apart.
 *
 * `.terminal-window` on its own matches both once the quake overlay has started
 * its session, and every assertion about "the" ring then becomes a strict-mode
 * violation the moment a test touches both.
 */
const slideWindow = (page: Page) =>
	page.locator(".toboggan-terminal .terminal-window");
const quakeWindow = (page: Page) =>
	page.locator(".toboggan-quake-inner .terminal-window");

/** How many terminals are wearing the ring. The claim is a single slot. */
async function ringCount(page: Page): Promise<number> {
	return page.locator(".terminal-window.terminal-has-keys").count();
}

/**
 * The attribute a test stamps on a canvas to tell a survivor from a fresh one.
 *
 * A restarted session builds a whole new canvas, so the tag going missing *is*
 * the restart. Reading it back as `null` therefore means "torn down", and `yes`
 * means "the very element we started with".
 */
const SURVIVOR_ATTR = "data-before-restart";

/** Stamps {@link SURVIVOR_ATTR} on a canvas so a later teardown becomes visible. */
async function tagCanvas(canvas: Locator) {
	await expect(canvas).toBeVisible({ timeout: 20_000 });
	await canvas.evaluate(
		(el, attr) => el.setAttribute(attr, "yes"),
		SURVIVOR_ATTR,
	);
}

/**
 * The index of the first slide whose title matches, as `GoTo` numbers them.
 *
 * `/api/talk` describes the deck the server *presents* — `hidden_in = ["web"]`
 * slides are already dropped from it — so this is the same numbering `GoTo`
 * takes, and no test has to hard-code a position the guide can renumber.
 */
async function slideIndex(
	request: APIRequestContext,
	pattern: RegExp,
): Promise<number> {
	const talk = await (await request.get("/api/talk")).json();
	const index = (talk.titles as (string | null)[]).findIndex((title) =>
		pattern.test(title ?? ""),
	);
	expect(index, `the deck under test has no ${pattern} slide`).toBeGreaterThan(
		-1,
	);
	return index;
}

/**
 * Which slide the client believes it is on, 1-based.
 *
 * The app writes it to `--current-slide` on its `<main>` host on every state
 * change, which makes it the one place a test can read the client's own idea of
 * where the deck is — as opposed to the server's, which is ahead of it.
 */
async function currentSlideNumber(page: Page): Promise<number> {
	return page.evaluate(() => {
		const host = document.querySelector("main");
		const raw = host?.style.getPropertyValue("--current-slide") ?? "";
		return Number.parseInt(raw, 10);
	});
}

/**
 * Moves the deck to the first slide whose title matches and waits for it to land.
 *
 * Waiting is not politeness: `POST /api/command` returns as soon as the server
 * has the command, and the client applies the broadcast state a round trip
 * later. A test that opened the quake overlay in between opened it against the
 * *old* slide's cwd, and the state then arriving restarted the session — which
 * is exactly the behaviour under test here, so the race read as a real failure.
 */
async function goToSlide(
	page: Page,
	request: APIRequestContext,
	pattern: RegExp,
): Promise<number> {
	const index = await slideIndex(request, pattern);
	await request.post("/api/command", {
		data: { command: "GoTo", slide: index },
	});
	await expect
		.poll(() => currentSlideNumber(page), {
			message: `the deck never reached slide ${index + 1}`,
		})
		.toBe(index + 1);
	return index;
}

/**
 * How many step markers the current slide has revealed.
 *
 * The proof that a `NextStep` actually landed. Polling on this rather than
 * sleeping is what keeps "the session survived" from passing simply because the
 * command had not arrived yet.
 */
async function revealedSteps(page: Page): Promise<number> {
	return page.evaluate(() => {
		const host = [...document.querySelectorAll("*")].find((element) =>
			element.shadowRoot?.querySelector("section"),
		);
		return host?.shadowRoot?.querySelectorAll(".step-current").length ?? 0;
	});
}

/**
 * Enough of the current slide to tell it apart from the next one.
 *
 * The deck marks nothing with its index, and `textContent` does not cross shadow
 * boundaries — so this reads the slide's own text and none of the terminal's.
 */
async function slideFingerprint(page: Page): Promise<string> {
	return page.evaluate(() => {
		const host = [...document.querySelectorAll("*")].find((element) =>
			element.shadowRoot?.querySelector("section"),
		);
		const section = host?.shadowRoot?.querySelector("section");
		return (section?.textContent ?? "")
			.replace(/\s+/g, " ")
			.trim()
			.slice(0, 120);
	});
}

/**
 * The box the layout gives an element, in CSS pixels.
 *
 * `offsetWidth`/`offsetHeight` rather than `boundingBox()`: every slide change
 * runs a 0.7s entrance animation that scales `.toboggan-slide` from 0.96 to 1,
 * so painted geometry is a frame of that animation as often as not. Whether
 * restoring puts the window back in its slot is a question about layout.
 */
async function layoutBox(locator: Locator): Promise<{ w: number; h: number }> {
	return locator.evaluate((el: HTMLElement) => ({
		w: el.offsetWidth,
		h: el.offsetHeight,
	}));
}

/** Asserts the deck did not move, giving it a moment to move if it were going to. */
async function expectDeckStill(page: Page, before: string) {
	await page.waitForTimeout(SETTLE_MS);
	expect(await slideFingerprint(page)).toBe(before);
}

/** Asserts the deck did move. */
async function expectDeckMoved(page: Page, before: string) {
	await expect
		.poll(() => slideFingerprint(page), { message: "the deck never advanced" })
		.not.toBe(before);
}

test.beforeEach(async ({ page, request }) => {
	await page.goto("/run");
	await page.waitForSelector("section", { timeout: 20_000 });
	// A terminal slide with no steps of its own: every test below that presses
	// `space` expecting the *deck* to move needs the press to reach a slide
	// change rather than be swallowed by a `<!-- pause -->`.
	await goToSlide(page, request, /terminal/i);

	// The canvas only exists once rioterm has loaded its own wasm and mounted.
	await expect(page.locator(".terminal-canvas")).toBeVisible({
		timeout: 20_000,
	});
});

test("typing at the shell does not drive the presentation", async ({
	page,
}) => {
	await page.locator(".terminal-canvas").click();

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckStill(page, before);
	// And the presenter can see why: the ring says who has the keys.
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	// The positive control. Without it a broken socket, a wasm panic after boot
	// or a deleted `space` binding would all make the assertion above pass.
	await page.keyboard.press("Shift+Escape");
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("clicking the title bar hands the keyboard to the shell", async ({
	page,
}) => {
	// The reported bug. rioterm focuses its hidden textarea only from a mousedown
	// on its own canvas container, so a click on the chrome left the terminal
	// looking like the thing being typed into while `space` advanced the deck.
	await page.locator(".terminal-titlebar").click();

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckStill(page, before);
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	// Positive control, as above.
	await page.keyboard.press("Shift+Escape");
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("clicking off the terminal gives the deck its keys back", async ({
	page,
}) => {
	await page.locator(".terminal-titlebar").click();
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	// Top-left of the viewport: on the slide, clear of the terminal pane.
	await page.mouse.click(5, 5);
	await expect(slideWindow(page)).not.toHaveClass(/terminal-has-keys/);

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("Shift+Escape gives the deck its keys back", async ({ page }) => {
	await page.locator(".terminal-canvas").click();
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	await page.keyboard.press("Shift+Escape");
	await expect(slideWindow(page)).not.toHaveClass(/terminal-has-keys/);

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("maximize and restore leave the window as they found it", async ({
	page,
}) => {
	const terminalWindow = slideWindow(page);
	const titlebar = page.locator(".terminal-titlebar");

	const windowBefore = await layoutBox(terminalWindow);
	const titlebarBefore = await layoutBox(titlebar);
	expect(windowBefore.h).toBeGreaterThan(0);
	expect(titlebarBefore.h).toBeGreaterThan(0);

	await page.locator(".terminal-btn-maximize").click();

	// The top layer, not a move in the DOM: the window is still where it was
	// rendered, which is what keeps every selector that styles it matching.
	await expect
		.poll(() => terminalWindow.evaluate((el) => el.matches(":popover-open")))
		.toBe(true);
	const viewport = page.viewportSize();
	const maximized = await layoutBox(terminalWindow);
	expect(maximized.w).toBe(viewport?.width);
	expect(maximized.h).toBe(viewport?.height);

	await page.locator(".terminal-btn-minimize").click();

	// The attribute goes too, or the UA popover rules keep dressing the window.
	await expect(terminalWindow).not.toHaveAttribute("popover", /.*/);
	expect(await layoutBox(terminalWindow)).toEqual(windowBefore);

	// The decoration is back: the title bar at the height it had, corners rounded.
	expect(await layoutBox(titlebar)).toEqual(titlebarBefore);
	const radius = await terminalWindow.evaluate(
		(el) => getComputedStyle(el).borderTopLeftRadius,
	);
	expect(radius).not.toBe("0px");
});

test("the quake overlay owns the keyboard while it is down", async ({
	page,
}) => {
	const overlay = page.locator(".toboggan-quake-terminal");

	await page.keyboard.press("Backquote");
	await expect(overlay).toHaveClass(/open/);
	// Not just "the deck is muted" — the old global flag did that too. The claim
	// has to be held by the overlay's own terminal, by the same route a slide's
	// terminal uses.
	await expect(quakeWindow(page)).toHaveClass(/terminal-has-keys/);

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckStill(page, before);

	await page.keyboard.press("Backquote");
	await expect(overlay).not.toHaveClass(/open/);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("bare Escape stays with the shell", async ({ page }) => {
	// The chord is `Shift`+`Escape` precisely so that `Escape` itself keeps
	// working: a terminal on a slide exists to run `vim`, `less` and friends.
	// Drop the shift test and every one of those demos breaks.
	await page.locator(".terminal-canvas").click();
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	await page.keyboard.press("Escape");
	await page.waitForTimeout(SETTLE_MS);
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);
});

test("maximizing re-fits the shell to the bigger window", async ({ page }) => {
	// The window growing is not the point — the grid following it is. `refit`
	// returning `None` makes a silent no-op newly possible here, and a maximized
	// window still running an 80-column shell looks like a styling bug.
	const canvas = page.locator(".terminal-canvas");
	const before = await layoutBox(canvas);
	expect(before.w).toBeGreaterThan(0);

	await page.locator(".terminal-btn-maximize").click();
	await expect
		.poll(async () => (await layoutBox(canvas)).w)
		.toBeGreaterThan(before.w);

	await page.locator(".terminal-btn-minimize").click();
	await expect.poll(async () => (await layoutBox(canvas)).w).toBe(before.w);
});

test("opening the quake overlay takes the keys off a slide terminal", async ({
	page,
}) => {
	// The single slot, and the only scenario the owner id exists for.
	await page.locator(".terminal-titlebar").click();
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	await page.keyboard.press("Backquote");
	await expect(page.locator(".toboggan-quake-terminal")).toHaveClass(/open/);
	await expect(quakeWindow(page)).toHaveClass(/terminal-has-keys/);
	await expect(slideWindow(page)).not.toHaveClass(/terminal-has-keys/);
	expect(await ringCount(page), "two terminals cannot both hold the keys").toBe(
		1,
	);
});

test("leaving a terminal slide gives the deck its keys back", async ({
	page,
	request,
}) => {
	// The worst failure in this feature's blast radius: a claim that outlives its
	// terminal leaves the presenter with a deck that answers nothing, mid-talk,
	// with no gesture that fixes it.
	await page.locator(".terminal-titlebar").click();
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	// Driven from the API, not a keypress: a keypress would be swallowed by the
	// very bug under test, and the test would fail for the wrong reason.
	await request.post("/api/command", { data: { command: "GoTo", slide: 0 } });
	await expect(page.locator(".terminal-canvas")).toHaveCount(0);
	expect(
		await ringCount(page),
		"the terminal took the keyboard to the grave",
	).toBe(0);

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("a step advance leaves a slide's own terminal alone", async ({
	page,
	request,
}) => {
	// The same defect, reached by a different route: `set_slide` rebuilt the
	// slide on every state change, and rebuilding stops every terminal on it. A
	// deck that pairs `<!-- pause -->` with `<!-- term: . | cargo watch -->`
	// restarted the command each time the presenter stepped through the slide.
	await goToSlide(page, request, /live demo/i);
	const canvas = page.locator(".toboggan-terminal .terminal-canvas");
	await tagCanvas(canvas);
	expect(
		await revealedSteps(page),
		"this slide needs an unrevealed step for the advance to land on",
	).toBe(0);

	await request.post("/api/command", { data: { command: "NextStep" } });
	await expect
		.poll(() => revealedSteps(page), {
			message: "the step never landed, so this proves nothing",
		})
		.toBeGreaterThan(0);

	expect(
		await canvas.getAttribute(SURVIVOR_ATTR),
		"the step advance restarted the slide's terminal",
	).toBe("yes");
});

test("a step advance leaves the quake session alone", async ({
	page,
	request,
}) => {
	// The whole point of the overlay: a build, a `tail -f`, a REPL — started once
	// and still running three slides later. The deck re-sends its state on every
	// step, and a restart on each one killed whatever was being demoed while the
	// overlay sat there looking untouched, because it is hidden by a transform
	// rather than by teardown.
	//
	// Driven from the API: while the overlay is down it owns the keyboard, so a
	// keypress would go to the shell instead of the deck.
	//
	// Deliberately a slide with *no* `quake_cwd` of its own — the overwhelmingly
	// common case, and the only one the defect showed up in: the overlay compared
	// the resolved cwd it was running in against the raw `Option` the slide
	// carries, and `Some(".")` never equals `None`. On a slide that does set one,
	// the two agreed by accident and nothing restarted.
	await goToSlide(page, request, /presenter view/i);
	await page.keyboard.press("Backquote");
	const overlay = page.locator(".toboggan-quake-terminal");
	await expect(overlay).toHaveClass(/open/);

	const canvas = page.locator(".toboggan-quake-inner .terminal-canvas");
	await tagCanvas(canvas);
	expect(
		await revealedSteps(page),
		"this slide needs an unrevealed step for the advance to land on",
	).toBe(0);

	await request.post("/api/command", { data: { command: "NextStep" } });
	await expect
		.poll(() => revealedSteps(page), {
			message: "the step never landed, so this proves nothing",
		})
		.toBeGreaterThan(0);

	expect(
		await canvas.getAttribute(SURVIVOR_ATTR),
		"the step advance restarted the shell",
	).toBe("yes");
	await expect(overlay).toHaveClass(/open/);
	await expect(quakeWindow(page)).toHaveClass(/terminal-has-keys/);
});

test("a slide change to the same cwd leaves the quake session alone", async ({
	page,
	request,
}) => {
	// Same invariant one step out: the overlay's cwd is re-resolved on every
	// slide, and almost every deck resolves every slide to the same directory.
	// The comparison used to be between a resolved cwd and a raw `Option`, so
	// "unchanged" read as "changed" for all of them.
	await page.keyboard.press("Backquote");
	await expect(page.locator(".toboggan-quake-terminal")).toHaveClass(/open/);
	const canvas = page.locator(".toboggan-quake-inner .terminal-canvas");
	await tagCanvas(canvas);

	await goToSlide(page, request, /presenter view/i);

	expect(
		await canvas.getAttribute(SURVIVOR_ATTR),
		"a slide that changed nothing restarted the shell",
	).toBe("yes");
	await expect(quakeWindow(page)).toHaveClass(/terminal-has-keys/);
});

test("the quake overlay keeps its keys across a session restart", async ({
	page,
	request,
}) => {
	// A slide that names a *different* `quake_cwd` does restart the session — and
	// tearing a session down releases its claim. If nothing re-claims, the deck
	// arms itself under an overlay that is still down, and the next `space`
	// restarts the very session being demoed.
	await page.keyboard.press("Backquote");
	const overlay = page.locator(".toboggan-quake-terminal");
	await expect(overlay).toHaveClass(/open/);
	await expect(quakeWindow(page)).toHaveClass(/terminal-has-keys/);

	// Tag the canvas so we can tell a real restart from a no-op.
	const canvas = page.locator(".toboggan-quake-inner .terminal-canvas");
	await tagCanvas(canvas);

	// The one slide in the guide with a `quake_cwd` of its own, which is what
	// makes this a genuine change of directory rather than a no-op.
	await goToSlide(page, request, /keyboard/i);
	await expect(overlay).toHaveClass(/open/);
	await expect
		.poll(() => canvas.getAttribute(SURVIVOR_ATTR), {
			message: "the session never restarted, so this proves nothing",
		})
		.toBe(null);

	// The overlay is still down, so its keys are still its own.
	await expect(quakeWindow(page)).toHaveClass(/terminal-has-keys/);
	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckStill(page, before);
});

test("the quake overlay survives a restart and a reopen", async ({
	page,
	request,
}) => {
	// The invariant that replaced `set_persistent`. The overlay reuses one host
	// for the life of the page, and `restart_session` tears its session down and
	// builds a new one inside it — so a teardown that took the host with it left
	// the overlay empty for the rest of the talk.
	//
	// Its own terminal cannot be maximized: the overlay hides the title bar with
	// `--terminal-titlebar-display: none`, so there are no traffic lights to
	// click. The reachable way to destroy the host is a real change of cwd.
	await page.keyboard.press("Backquote");
	const overlay = page.locator(".toboggan-quake-terminal");
	await expect(overlay).toHaveClass(/open/);
	const canvas = page.locator(".toboggan-quake-inner .terminal-canvas");
	await tagCanvas(canvas);

	await goToSlide(page, request, /keyboard/i);
	await expect
		.poll(() => canvas.getAttribute(SURVIVOR_ATTR), {
			message: "the session never restarted, so this proves nothing",
		})
		.toBe(null);
	await page.keyboard.press("Backquote");
	await expect(overlay).not.toHaveClass(/open/);
	await page.keyboard.press("Backquote");
	await expect(overlay).toHaveClass(/open/);

	// Still a terminal, and still filling the overlay rather than collapsing to
	// the height of its own content — the shape the old reparenting destroyed.
	await expect(canvas).toBeVisible();
	const inner = await layoutBox(page.locator(".toboggan-quake-inner"));
	expect(inner.h).toBeGreaterThan(0);
	expect(inner.w).toBeGreaterThan(0);
	const overlayBox = await layoutBox(overlay);
	expect(inner.h).toBe(overlayBox.h);
});

test("Shift+Escape takes the quake overlay up with the keys", async ({
	page,
}) => {
	// An overlay that is down but no longer listening is a trap: it covers the
	// slide, shows a live shell, and `space` quietly drives the presentation.
	await page.keyboard.press("Backquote");
	const overlay = page.locator(".toboggan-quake-terminal");
	await expect(overlay).toHaveClass(/open/);

	await page.keyboard.press("Shift+Escape");
	await expect(overlay).not.toHaveClass(/open/);
	expect(await ringCount(page)).toBe(0);
});

test("blanking the screen covers a maximized terminal", async ({ page }) => {
	// A maximized terminal is in the top layer, which paints above every z-index
	// there is. Blanking is the one control whose whole job is to hide what is on
	// screen, and the terminal is the likeliest thing to be showing something the
	// room should not see.
	await page.locator(".terminal-btn-maximize").click();
	await expect
		.poll(() => slideWindow(page).evaluate((el) => el.matches(":popover-open")))
		.toBe(true);

	// Hand the keys back, or the deck will not answer the blank key at all.
	await page.keyboard.press("Shift+Escape");
	await page.keyboard.press(".");

	await expect
		.poll(
			() =>
				page.evaluate(() => {
					const hit = document.elementFromPoint(
						Math.floor(window.innerWidth / 2),
						Math.floor(window.innerHeight / 2),
					);
					return hit?.id ?? hit?.tagName ?? "none";
				}),
			{ message: "the terminal is still painting over the blank overlay" },
		)
		.toBe("toboggan-blank");
});

test("a terminal whose shell has exited does not mute the deck", async ({
	page,
}) => {
	// A claim mutes the deck, so a session that can no longer take a keystroke
	// must not be given one: the presenter would be left clicking a dead terminal
	// that wears the ring while the deck stops answering.
	//
	// This needs the server to notice the shell died — nothing else does, and
	// until it did, the browser went on believing the session was live.
	await page.locator(".terminal-canvas").click();
	await expect(slideWindow(page)).toHaveClass(/terminal-has-keys/);

	// `type` inserts text; the newline has to be a real Enter or the shell never
	// sees a command at all.
	await page.keyboard.type("exit");
	await page.keyboard.press("Enter");
	await expect(slideWindow(page)).not.toHaveClass(/terminal-has-keys/, {
		timeout: 10_000,
	});

	await page.locator(".terminal-titlebar").click();
	await expect(
		slideWindow(page),
		"a dead terminal took the keyboard anyway",
	).not.toHaveClass(/terminal-has-keys/);

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});
