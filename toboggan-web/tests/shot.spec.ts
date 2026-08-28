import { expect, test } from "@playwright/test";

/**
 * `/run?shot=N` — the page a headless browser photographs to illustrate the
 * slide overview.
 *
 * Everything asserted here is a contract the Rust driver in
 * `toboggan-server/src/services/shots.rs` depends on and cannot check for
 * itself: the Rust gate stubs `toboggan-web/dist`, so it proves nothing about
 * the page. The failure mode without these tests is the quiet one — an overview
 * full of blank or half-built cards that still exits 0.
 */

/** The attribute the driver polls. Must match `SHOT_READY_ATTRIBUTE`. */
const READY = "data-toboggan-shot";

test("a shot announces itself ready", async ({ page }) => {
	await page.goto("/run?shot=1");
	await expect(page.locator("html")).toHaveAttribute(READY, "ready", {
		timeout: 15_000,
	});
	await expect(page.locator("section").first()).toBeVisible();
});

test("a shot shows every reveal at once", async ({ page, request }) => {
	// A thumbnail of a half-built slide is a thumbnail of nothing, so every
	// reveal must be shown — `step-done` is the class that says so.
	//
	// The slide is looked up rather than hard-coded: which index carries the
	// guide's `<!-- pause -->` markers is a fact about the guide, and a test that
	// pins it fails the next time a slide is inserted before it.
	const { slides } = await (await request.get("/api/slides")).json();
	const index = slides.findIndex((slide: { body?: { raw?: string } }) =>
		slide.body?.raw?.includes('class="step'),
	);
	expect(index, "the guide has a slide with reveals").toBeGreaterThanOrEqual(0);

	await page.goto(`/run?shot=${index}`);
	await expect(page.locator("html")).toHaveAttribute(READY, "ready", {
		timeout: 15_000,
	});

	const total = await page.locator(".step").count();
	expect(total).toBeGreaterThan(0);
	await expect(page.locator(".step.step-done")).toHaveCount(total);
});

test("a shot registers no client and moves nothing", async ({
	page,
	request,
}) => {
	// The whole reason the shot page paints itself from REST rather than reusing
	// `/run?slide=N`: that path sends a `GoTo` command, so photographing a deck
	// would walk the room through it. A shot must be invisible to the talk.
	const before = await (await request.get("/api/clients")).json();

	await page.goto("/run?shot=2");
	await expect(page.locator("html")).toHaveAttribute(READY, "ready", {
		timeout: 15_000,
	});

	const after = await (await request.get("/api/clients")).json();
	expect(after.clients.length).toBe(before.clients.length);
});

test("a shot holds still", async ({ page }) => {
	// `state.css` gives `.running .toboggan-slide` a 0.7s entrance that
	// translates, scales and blurs. A screenshot taken during it is a smeared
	// slide sliding in from the right, so the shot page neutralises it — and the
	// step transition inside the shadow root, which no document rule can reach.
	await page.goto("/run?shot=1");
	await expect(page.locator("html")).toHaveAttribute(READY, "ready", {
		timeout: 15_000,
	});

	const animated = await page.evaluate(
		() =>
			document.getAnimations().filter((a) => a.playState === "running").length,
	);
	expect(animated).toBe(0);

	const stepDuration = await page.evaluate(() =>
		getComputedStyle(document.documentElement).getPropertyValue(
			"--step-transition-duration",
		),
	);
	expect(stepDuration.trim()).toBe("0s");
});

test("a shot of a slide that does not exist reports an error", async ({
	page,
}) => {
	// The value that keeps a failed fetch out of the overview: without it the
	// driver waits for `ready`, times out, and either files a blank rectangle or
	// blames the timeout rather than the missing slide.
	await page.goto("/run?shot=99999");
	await expect(page.locator("html")).toHaveAttribute(READY, "error", {
		timeout: 15_000,
	});
});

test("a shot at a viewport of 1280x720 fills the frame", async ({ page }) => {
	// The size the driver captures at, and the size the presenter view lays its
	// mirrors out at. A deck breaks its lines against the viewport it is given,
	// so a preview at another size is a preview of a different slide.
	await page.setViewportSize({ width: 1280, height: 720 });
	await page.goto("/run?shot=1");
	await expect(page.locator("html")).toHaveAttribute(READY, "ready", {
		timeout: 15_000,
	});

	const slide = page.locator(".toboggan-slide").first();
	const box = await slide.boundingBox();
	expect(box).not.toBeNull();
	// Wide enough to be the deck rather than a collapsed shell. Measured after
	// `ready`, which is what makes the number trustworthy: a box read mid
	// entrance-animation is a scaled, translated lie.
	expect(box?.width ?? 0).toBeGreaterThan(1_000);
});
