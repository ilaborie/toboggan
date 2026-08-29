import { expect, type Frame, type Page, test } from "@playwright/test";

/**
 * The presenter view, and the one claim it exists to make: that the pane shows
 * what the room shows.
 *
 * It is an iframe of `/run` now, painted by `postMessage`. It used to be the
 * slide component re-rendered into the presenter's own shadow tree and shrunk
 * with CSS `zoom`, which could not be faithful — the slide inherited the
 * chrome's 16px base rather than the deck's viewport-derived one, so every type
 * size, which is a percentage, came out at half. The assertions below are
 * written against that failure: the *ratios* matter more than the text.
 *
 * The *next* slide is not a mirror but a photograph — the still the slide
 * overview took of it — so there is exactly one deck in this page now.
 *
 * The whole suite is serial against one server holding one presentation state,
 * so every test here puts the deck where it wants it and assumes nothing about
 * where the last one left it.
 */

const PRESENTER_BG = "rgb(13, 17, 23)";

/** The logical viewport every mirror is laid out in. */
const STAGE = { width: 1280, height: 720 };

/** Puts the deck on `slide` without going through a client. */
const goTo = async (page: Page, slide: number) => {
	const response = await page.request.post("/api/command", {
		data: { command: "GoTo", slide },
	});
	expect(response.ok()).toBe(true);
};

/**
 * The frame behind one pane.
 *
 * By `page.frames()` rather than a `FrameLocator`, because the assertions need
 * `evaluate` — computed styles inside the mirror's own document are the point —
 * and a `FrameLocator` has none.
 */
const mirror = async (page: Page): Promise<Frame> => {
	const url = "mirror=current";
	await expect
		.poll(() => page.frames().some((frame) => frame.url().includes(url)), {
			timeout: 15_000,
		})
		.toBe(true);
	const frame = page.frames().find((f) => f.url().includes(url));
	if (!frame) {
		throw new Error("No current-slide mirror");
	}
	// The slide arrives by message, a moment after the document loads.
	await frame.waitForSelector(".toboggan-slide", { timeout: 15_000 });
	await expect
		.poll(
			() =>
				frame.evaluate(
					() =>
						document
							.querySelector(".toboggan-slide")
							?.shadowRoot?.querySelector("section")?.childElementCount ?? 0,
				),
			{ timeout: 15_000 },
		)
		.toBeGreaterThan(0);
	return frame;
};

/** What a slide looks like from inside whichever document is rendering it. */
const measure = (frame: Frame | Page) =>
	frame.evaluate(() => {
		const section = document
			.querySelector(".toboggan-slide")
			?.shadowRoot?.querySelector("section");
		const steps = [...(section?.querySelectorAll(".step") ?? [])];
		return {
			title: section?.querySelector("h2")?.textContent?.trim() ?? "",
			rootFontSize: getComputedStyle(document.documentElement).fontSize,
			viewportWidth: document.documentElement.clientWidth,
			headTags: document.querySelectorAll("head [data-toboggan-head]").length,
			steps: steps.length,
			revealed: steps.filter((step) => step.classList.contains("step-done"))
				.length,
		};
	});

test("a pane lays the deck out exactly as the deck does", async ({ page }) => {
	await goTo(page, 6);

	// The deck at the mirror's own viewport, which is what a mirror claims to be.
	await page.setViewportSize(STAGE);
	await page.goto("/run");
	await page.waitForSelector(".toboggan-slide");
	const deck = await measure(page);

	// The presenter at a size nothing like it. The pane is still 1280x720 inside
	// and painted through a transform, so the root size — and with it every
	// heading, every code block, every line break — must be identical. Sized to
	// its pane instead, it would report something smaller here, which is exactly
	// what the `zoom` this replaced did.
	await page.setViewportSize({ width: 1600, height: 1000 });
	await page.goto("/presenter");
	const now = await measure(await mirror(page));

	expect(now.viewportWidth).toBe(STAGE.width);
	expect(now.rootFontSize).toBe(deck.rootFontSize);
	expect(now.title).toBe(deck.title);
});

test("the deck's own CSS stays inside the panes", async ({ page }) => {
	await goTo(page, 6);
	await page.goto("/presenter");
	const now = await mirror(page);

	// The guide's `_head.html` pulls a stylesheet that paints `main` — and
	// `<main>` is this shell's shadow host, so injected here it repainted the
	// speaker's chrome with the projector's backdrop.
	expect((await measure(now)).headTags).toBeGreaterThan(0);
	await expect(page.locator("head [data-toboggan-head]")).toHaveCount(0);
	await expect(page.locator("main")).toHaveCSS(
		"background-color",
		PRESENTER_BG,
	);
	// And the deck's `html { font-size: clamp(…) }` does not retune the chrome.
	await expect(page.locator("html")).toHaveCSS("font-size", "16px");
});

test("the next pane names and photographs the slide after this one", async ({
	page,
}) => {
	const talk = await (await page.request.get("/api/talk")).json();
	// A slide whose successor has a title to name.
	const index = talk.titles.findIndex(
		(title: string, at: number) => at > 0 && title,
	);
	expect(index).toBeGreaterThan(0);

	await goTo(page, index - 1);
	await page.goto("/presenter");

	await expect(page.locator(".next-title")).toHaveText(talk.titles[index], {
		timeout: 15_000,
	});
	await expect(page.locator(".next-number")).toHaveText(String(index + 1));

	// A still of the deck, addressed by the *presented* index — the server
	// crosses to the authored one behind the route.
	const shot = page.locator(".next-shot");
	await expect(shot).toHaveAttribute(
		"src",
		new RegExp(`^/overview/slide/${index}\\?`),
		// The deck is photographed as the server starts, and photographing one
		// takes seconds — so a run that opens this page immediately can still be
		// waiting on it.
		{ timeout: 60_000 },
	);
	// Decoded, not merely addressed: a `503` sets the attribute perfectly well.
	await expect
		.poll(() => shot.evaluate((img) => (img as HTMLImageElement).naturalWidth))
		.toBeGreaterThan(0);

	// And no second deck: the pane used to be a whole wasm client in an iframe.
	await expect(page.locator('iframe[src*="mirror=next"]')).toHaveCount(0);
});

test("the next pane clears at the end of the deck", async ({ page }) => {
	const talk = await (await page.request.get("/api/talk")).json();
	await goTo(page, talk.titles.length - 1);
	await page.goto("/presenter");

	await expect(page.locator(".counter")).toContainText(
		`${talk.titles.length}/`,
		{ timeout: 15_000 },
	);
	// Nothing to come: no number, no title, and no picture behind them.
	await expect(page.locator(".next-number")).toHaveText("");
	await expect(page.locator(".next-title")).toHaveText("");
	await expect(page.locator(".next-shot")).not.toHaveAttribute("src", /./);
});

test("the on-screen navigation drives the deck", async ({ page, context }) => {
	await goTo(page, 6);

	// A second window on the deck, because the claim is that the button reaches
	// the server — not merely that it redraws the pane beside it.
	const room = await context.newPage();
	await room.goto("/run");
	await room.waitForSelector(".toboggan-slide");

	await page.goto("/presenter");
	await expect(page.locator(".counter")).toHaveText("7/43", {
		timeout: 15_000,
	});

	await page.locator(".go-next").click();
	await expect(page.locator(".counter")).toHaveText("8/43");
	await expect
		.poll(() =>
			room.evaluate(() =>
				document
					.querySelector("main")
					?.style.getPropertyValue("--current-slide"),
			),
		)
		.toBe("8");

	await page.locator(".go-prev").click();
	await expect(page.locator(".counter")).toHaveText("7/43");
	await room.close();
});

test("a pane opens no socket and starts no shell", async ({ page }) => {
	const sockets: string[] = [];
	// Child frames included, so a socket opened from inside a mirror is caught.
	page.on("websocket", (socket) => sockets.push(socket.url()));

	// The slide the guide declares a live terminal on.
	const talk = await (await page.request.get("/api/talk")).json();
	const index = talk.titles.findIndex((title: string) => /term\b/.test(title));
	expect(index).toBeGreaterThan(-1);
	await goTo(page, index);

	await page.goto("/presenter");
	await mirror(page);
	await page.waitForTimeout(500);

	// A second set of terminals would be a second set of shells, in a second
	// session, showing the room output it never asked for.
	expect(sockets.filter((url) => url.includes("/api/terminal"))).toEqual([]);
	// And exactly one client registers: the view itself. That a mirror never
	// registers is also what stops a framed `/run` — loopback, and so granted the
	// presenter role at the handshake — from being able to drive the deck.
	expect(sockets.filter((url) => url.includes("/api/ws"))).toHaveLength(1);
});

test("the timer pauses and resets", async ({ page }) => {
	await goTo(page, 6);
	await page.goto("/presenter");
	const elapsed = page.locator(".elapsed");
	await expect(elapsed).toBeVisible({ timeout: 15_000 });

	await page.locator(".pause").click();
	await expect(page.locator(".pause")).toHaveText("▶");
	const held = await elapsed.textContent();
	await page.waitForTimeout(2_200);
	expect(await elapsed.textContent()).toBe(held);

	await page.locator(".reset").click();
	await expect(elapsed).toHaveText("⏱ 0:00");
});

/**
 * Not covered, deliberately: the audience path that hides the navigation.
 *
 * Playwright reaches the server over loopback, and the server grants every
 * loopback peer the presenter role unconditionally — there is no token that can
 * demote it. `set_can_drive(false)` is reachable only from a non-loopback bind,
 * which this harness does not do.
 */
test("the navigation is shown to a client that may present", async ({
	page,
}) => {
	await page.goto("/presenter");
	await expect(page.locator(".layout")).toHaveAttribute(
		"data-role",
		"presenter",
		{ timeout: 15_000 },
	);
	await expect(page.locator(".go-next")).toBeVisible();
});
