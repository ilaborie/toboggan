import { expect, test } from "@playwright/test";

/**
 * Every page the server serves, and enough of each to know it rendered.
 *
 * A 200 is not the assertion that matters here: the deck is compiled Rust
 * behind a shadow root, so a page can answer 200 with a blank body when the
 * wasm bundle fails to boot. Each test therefore waits for something the
 * client itself had to produce.
 */

test("the homepage links to the deck", async ({ page }) => {
	await page.goto("/");
	await expect(
		page.getByRole("link", { name: /present/i }).first(),
	).toBeVisible();
});

test("the deck renders a slide", async ({ page }) => {
	await page.goto("/run");
	// `section` lives inside the slide component's shadow root; Playwright
	// pierces open shadow roots, so this only passes if wasm booted and the
	// component rendered.
	await expect(page.locator("section").first()).toBeVisible({
		timeout: 15_000,
	});
});

test("the presenter view shows its status strip", async ({ page }) => {
	await page.goto("/presenter");
	await expect(page.locator(".status .counter")).toBeVisible({
		timeout: 15_000,
	});
	await expect(page.locator(".now .fit")).toBeVisible();
});

test("the guide is served with any deck", async ({ page }) => {
	const response = await page.goto("/guide");
	expect(response?.status()).toBe(200);
	await expect(page.locator("section").first()).toBeVisible({
		timeout: 15_000,
	});
});

test("the slide overview answers", async ({ page }) => {
	const response = await page.goto("/slides");
	expect(response?.status()).toBe(200);
});

test("the API describes the deck", async ({ request }) => {
	const response = await request.get("/api/talk");
	expect(response.status()).toBe(200);
	const talk = await response.json();
	expect(talk.titles.length).toBeGreaterThan(0);
});

test("the deck downloads as a PDF", async ({ request }) => {
	const response = await request.get("/download.pdf");
	expect(response.status()).toBe(200);
	expect(response.headers()["content-type"]).toContain("pdf");
	// A typst failure used to surface as a 503 with an empty body; a PDF that
	// exists but is a few bytes long would be just as broken.
	expect((await response.body()).byteLength).toBeGreaterThan(10_000);
});

test("the API reference is served", async ({ request }) => {
	const response = await request.get("/doc");
	expect(response.status()).toBe(200);
});
