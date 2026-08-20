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
 *
 * Run in series. `POST /api/command` moves the *shared* deck — there is one
 * server and one presentation state for every test in the run — so under the
 * config's `fullyParallel` these three viewports interleaved their `GoTo`s and
 * each measured whichever slide another test had just navigated to.
 */
test.describe.configure({ mode: "serial" });

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
		// The deck's own faces have to be in before anything is measured. A slide
		// laid out with fallback metrics is a different height from the same
		// slide laid out with the font it ships, and only one of the two is the
		// thing being asserted about.
		await page.evaluate(() => document.fonts.ready);

		const overflowing: string[] = [];
		for (let index = 0; index < total; index++) {
			await request.post("/api/command", {
				data: { command: "GoTo", slide: index },
			});

			// Polled rather than slept: a fixed 80ms was both slower than the
			// usual render and, on a loaded CI runner, occasionally shorter than
			// it — which measured the previous slide and called it this one.
			// The title is what identifies the slide that actually rendered; the
			// deck marks nothing else with its index.
			const measure = () =>
				page.evaluate(async () => {
					// Again per slide, not just once for the deck: `fonts.ready`
					// resolves for the faces loading at the time, and a slide that
					// is the first to want a weight starts a fresh load. Cheap when
					// there is nothing outstanding, which is the usual case.
					await document.fonts.ready;
					const host = [...document.querySelectorAll("*")].find((element) =>
						element.shadowRoot?.querySelector("section"),
					);
					const section = host?.shadowRoot?.querySelector("section");
					if (!section) return null;
					return {
						title: section.querySelector("h2")?.textContent?.trim() ?? "",
						x: section.scrollWidth - section.clientWidth,
						y: section.scrollHeight - section.clientHeight,
					};
				});

			const expected = (talk.titles[index] ?? "").trim();
			if (expected) {
				await expect
					.poll(async () => (await measure())?.title, {
						message: `slide ${index + 1} never rendered`,
						timeout: 10_000,
					})
					.toBe(expected);
			} else {
				// A cover or a part slide may carry no <h2> to wait on.
				await page.waitForTimeout(80);
			}

			const overflow = await measure();
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
