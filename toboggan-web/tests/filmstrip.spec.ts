import { expect, type Page, test } from "@playwright/test";

/**
 * The presenter view's slide grid: every slide at once, and a way to jump.
 *
 * The Rust gate stubs `toboggan-web/dist`, so nothing on that side can see this
 * panel at all — and its two hardest parts are both invisible when they go
 * wrong. A grid built over the *authored* deck would show a card the speaker
 * can never reach and put every picture after a `hidden_in = ["web"]` slide
 * under the wrong number; a grid that never noticed the thumbnails had arrived
 * would sit on "Rendering slide previews…" for the length of the talk.
 */

/** Waits for the thumbnails, which are generated on first request. */
async function openStrip(page: Page) {
	await page.goto("/presenter");
	await page.locator(".strip-toggle").click();
	await expect(page.locator(".filmstrip")).toHaveAttribute(
		"data-previews",
		"ready",
		// The whole deck is photographed before the first cell can be filled.
		{ timeout: 60_000 },
	);
}

test("the grid has one cell per presented slide", async ({ page, request }) => {
	// Presented, not authored: `/api/slides` is the list the deck can be told to
	// go to, and a cell that cannot be reached is a button that does nothing.
	const { slides } = await (await request.get("/api/slides")).json();

	await openStrip(page);
	await expect(page.locator(".strip-cell")).toHaveCount(slides.length);
});

test("every cell shows a picture", async ({ page }) => {
	await openStrip(page);

	// `naturalWidth` rather than the `src` attribute: a `503` while the deck is
	// still being photographed sets the attribute perfectly well and renders a
	// broken image, which is the failure this whole probe-and-retry exists for.
	const undecoded = await page.evaluate(
		() =>
			[...document.querySelectorAll(".strip-cell img")].filter(
				(img) => !(img as HTMLImageElement).naturalWidth,
			).length,
	);
	expect(undecoded).toBe(0);
});

test("the current slide is marked", async ({ page }) => {
	await openStrip(page);
	await expect(page.locator('.strip-cell[aria-current="true"]')).toHaveCount(1);
});

test("clicking a cell jumps there and closes the grid", async ({ page }) => {
	await openStrip(page);

	// Far enough in that it cannot be where the deck already was.
	await page.locator(".strip-cell").nth(5).click();

	// Closed, because the grid covers the notes the speaker just jumped to read.
	await expect(page.locator(".filmstrip")).toBeHidden();
	await expect(page.locator(".counter")).toContainText("6/");
	await expect(
		page.locator('.strip-cell[aria-current="true"]'),
	).toHaveAttribute("data-slide", "5");
});

test("g opens the grid and Escape closes it", async ({ page }) => {
	await page.goto("/presenter");
	await expect(page.locator(".filmstrip")).toBeHidden();

	await page.keyboard.press("g");
	await expect(page.locator(".filmstrip")).toBeVisible();

	await page.keyboard.press("Escape");
	await expect(page.locator(".filmstrip")).toBeHidden();
});

test("the grid does not register a client of its own", async ({
	page,
	request,
}) => {
	// The thumbnails are photographed against a private server, so opening the
	// grid must not add a client to the room — the same promise the shot page
	// makes, checked from the other end.
	await openStrip(page);
	const { clients } = await (await request.get("/api/clients")).json();
	expect(clients.length).toBe(1);
});
