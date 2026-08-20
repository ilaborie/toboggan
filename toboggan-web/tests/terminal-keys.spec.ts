import { expect, type Page, test } from "@playwright/test";

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
 * How much of the viewport a maximized terminal must cover.
 *
 * Not an equality: the slide is transform-scaled to fit the screen, so every
 * box on it is measured in scaled viewport pixels rather than CSS pixels.
 */
const MAXIMIZED_COVERAGE = 0.95;

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
	await expect(page.locator(".terminal-window")).toHaveClass(
		/terminal-has-keys/,
	);
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
	await expect(page.locator(".terminal-window")).toHaveClass(
		/terminal-has-keys/,
	);
});

test("clicking off the terminal gives the deck its keys back", async ({
	page,
}) => {
	await page.locator(".terminal-titlebar").click();
	await expect(page.locator(".terminal-window")).toHaveClass(
		/terminal-has-keys/,
	);

	// Top-left of the viewport: on the slide, clear of the terminal pane.
	await page.mouse.click(5, 5);
	await expect(page.locator(".terminal-window")).not.toHaveClass(
		/terminal-has-keys/,
	);

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("Shift+Escape gives the deck its keys back", async ({ page }) => {
	await page.locator(".terminal-canvas").click();
	await expect(page.locator(".terminal-window")).toHaveClass(
		/terminal-has-keys/,
	);

	await page.keyboard.press("Shift+Escape");
	await expect(page.locator(".terminal-window")).not.toHaveClass(
		/terminal-has-keys/,
	);

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});

test("maximize and restore leave the window as they found it", async ({
	page,
}) => {
	const terminalWindow = page.locator(".terminal-window");
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

	const before = await slideFingerprint(page);
	await page.keyboard.press("Space");
	await expectDeckStill(page, before);

	await page.keyboard.press("Backquote");
	await expect(overlay).not.toHaveClass(/open/);
	await page.keyboard.press("Space");
	await expectDeckMoved(page, before);
});
