import { expect, type Locator, type Page, test } from "@playwright/test";

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
	const talk = await (await request.get("/api/talk")).json();
	const index = (talk.titles as (string | null)[]).findIndex((title) =>
		/terminal/i.test(title ?? ""),
	);
	expect(index, "the deck under test has no terminal slide").toBeGreaterThan(
		-1,
	);

	await page.goto("/run");
	await page.waitForSelector("section", { timeout: 20_000 });
	await request.post("/api/command", {
		data: { command: "GoTo", slide: index },
	});

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

test("the quake overlay keeps its keys across a slide change", async ({
	page,
	request,
}) => {
	// A slide change re-resolves the overlay's cwd and restarts its session, and
	// tearing a session down releases its claim. If nothing re-claims, the deck
	// arms itself under an overlay that is still down — and the next `space`
	// restarts the very session being demoed.
	await page.keyboard.press("Backquote");
	const overlay = page.locator(".toboggan-quake-terminal");
	await expect(overlay).toHaveClass(/open/);
	await expect(quakeWindow(page)).toHaveClass(/terminal-has-keys/);

	// Tag the canvas so we can tell a real restart from a no-op.
	await page
		.locator(".toboggan-quake-inner .terminal-canvas")
		.evaluate((el) => el.setAttribute("data-before-restart", "yes"));

	await request.post("/api/command", { data: { command: "GoTo", slide: 0 } });
	await expect(overlay).toHaveClass(/open/);
	await expect
		.poll(
			() =>
				page
					.locator(".toboggan-quake-inner .terminal-canvas")
					.getAttribute("data-before-restart"),
			{ message: "the session never restarted, so this proves nothing" },
		)
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
	// click. The reachable way to destroy the host is the restart.
	await page.keyboard.press("Backquote");
	const overlay = page.locator(".toboggan-quake-terminal");
	await expect(overlay).toHaveClass(/open/);
	const canvas = page.locator(".toboggan-quake-inner .terminal-canvas");
	await expect(canvas).toBeVisible();

	await request.post("/api/command", { data: { command: "GoTo", slide: 0 } });
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
