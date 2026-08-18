import { expect, test } from "@playwright/test";

/**
 * No slide may overflow its own frame.
 *
 * `section` clips with `overflow: hidden`, so a slide that is too tall does not
 * scroll or complain — it silently loses its last lines, and the first anyone
 * knows is when the room cannot read the end of a slide.
 *
 * Steps are hidden with `opacity`, not `display`, so a slide's layout at step 0
 * already includes every reveal. One measurement per slide is the worst case.
 */

/** 16:9 as decks are usually built, and the 4:3 laptop that has to survive it. */
const VIEWPORTS = [
	{ width: 1920, height: 1080 },
	{ width: 1280, height: 720 },
	{ width: 1024, height: 768 },
];

for (const viewport of VIEWPORTS) {
	const label = `${viewport.width}x${viewport.height}`;

	test(`no slide overflows at ${label}`, async ({ page, request, baseURL }) => {
		test.slow();
		await page.setViewportSize(viewport);

		const talk = await (await request.get("/api/talk")).json();
		const total = talk.titles.length;
		expect(total).toBeGreaterThan(0);

		await page.goto("/run");
		await page.waitForSelector("section", { timeout: 20_000 });

		const overflowing: string[] = [];
		for (let index = 0; index < total; index++) {
			await request.post("/api/command", {
				data: { command: "GoTo", slide: index },
			});
			await page.waitForTimeout(80);

			const overflow = await page.evaluate(() => {
				const host = [...document.querySelectorAll("*")].find((element) =>
					element.shadowRoot?.querySelector("section"),
				);
				const section = host?.shadowRoot?.querySelector("section");
				if (!section) return null;
				return {
					x: section.scrollWidth - section.clientWidth,
					y: section.scrollHeight - section.clientHeight,
				};
			});

			expect(overflow, `slide ${index + 1} did not render`).not.toBeNull();
			if (overflow && (overflow.x > 1 || overflow.y > 1)) {
				overflowing.push(
					`slide ${index + 1} "${talk.titles[index]}" ` +
						`overflows by ${overflow.x}x${overflow.y}px`,
				);
			}
		}

		expect(overflowing, `${baseURL} at ${label}`).toEqual([]);
	});
}

test("code is not boxed on hover", async ({ page }) => {
	await page.goto("/run");
	await page.waitForSelector("section", { timeout: 20_000 });

	// A `code` rule with a hover border matches the `code` inside every `pre`
	// too, so moving the mouse across a slide drew a rectangle around whole
	// code blocks — in front of the room.
	const hasHoverBorder = await page.evaluate(() => {
		const host = [...document.querySelectorAll("*")].find((element) =>
			element.shadowRoot?.querySelector("section"),
		);
		const sheets = [...(host?.shadowRoot?.styleSheets ?? [])];
		return sheets.some((sheet) =>
			[...sheet.cssRules].some(
				(rule) =>
					rule.cssText.includes(":hover") &&
					rule.cssText.includes("border") &&
					rule.cssText.includes("code"),
			),
		);
	});

	expect(hasHoverBorder).toBe(false);
});
