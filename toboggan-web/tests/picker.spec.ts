import { readFile, writeFile } from "node:fs/promises";
import { expect, type Page, test } from "@playwright/test";

/**
 * The presenter view's slide picker: every slide at once, a search box over
 * them, and a way to jump.
 *
 * The Rust gate stubs `toboggan-web/dist`, so nothing on that side can see this
 * dialog at all — and its hardest parts are all invisible when they go wrong. A
 * grid built over the *authored* deck would show a cell the speaker can never
 * reach and put every picture after a `hidden_in = ["web"]` slide under the
 * wrong number; a grid that never noticed the thumbnails had arrived would sit
 * on "Rendering slide previews…" for the length of the talk; and a search that
 * reads only the titles would answer "no slides" to a speaker who remembers
 * what they meant to *say* about one.
 */

type Entry = {
	title?: string;
	part?: string;
	text?: string;
	notes?: string;
};

/** Everything one slide can be found by, the way the picker joins it. */
const haystack = (slide: Entry) =>
	`${slide.title ?? ""} ${slide.part ?? ""} ${slide.text ?? ""} ${slide.notes ?? ""}`.toLowerCase();

/**
 * A word that finds exactly one slide, taken from `field` on that slide.
 *
 * Computed from the deck rather than written down, so editing the guide cannot
 * quietly turn "a body-only match is found" into a test that searches for a
 * word which is no longer there.
 */
const uniqueWord = (slides: Entry[], field: "text" | "notes") => {
	const elsewhere = (slide: Entry) =>
		field === "notes"
			? `${slide.title ?? ""} ${slide.text ?? ""}`.toLowerCase()
			: (slide.title ?? "").toLowerCase();
	// A slide's index is its position: the outline carries no index of its own,
	// because a second copy of a number the array already encodes is a second
	// thing that can be wrong.
	for (const [index, slide] of slides.entries()) {
		const words = (slide[field] ?? "").toLowerCase().match(/[a-z]{6,}/g) ?? [];
		for (const word of words) {
			if (elsewhere(slide).includes(word)) {
				continue;
			}
			const hits = slides.flatMap((other, at) =>
				haystack(other).includes(word) ? [at] : [],
			);
			if (hits.length === 1 && hits[0] === index) {
				return { word, index };
			}
		}
	}
	throw new Error(`No word unique to one slide's ${field}`);
};

/** Opens the picker, and waits for the thumbnails behind it. */
async function openPicker(page: Page) {
	await page.goto("/presenter");
	await page.locator(".strip-toggle").click();
	await expect(page.locator("dialog.picker")).toHaveAttribute(
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

	await openPicker(page);
	await expect(page.locator(".strip-cell")).toHaveCount(slides.length);
});

test("the outline is numbered the same way", async ({ request }) => {
	// The two lists are read against each other by position — cell N carries
	// outline entry N's text and jumps to slide N — and they are built in
	// different places, over differently filtered decks, from different fields.
	// Comparing the *titles* is what checks the alignment: comparing an index to
	// its own position in the array it came from proves nothing.
	const { slides } = await (await request.get("/api/slides")).json();
	const { titles } = await (await request.get("/api/talk")).json();
	const outline = await (await request.get("/api/outline")).json();

	expect(outline.slides).toHaveLength(slides.length);
	expect(outline.slides.map((slide: Entry) => slide.title ?? "")).toEqual(
		titles,
	);
});

test("every cell shows a picture", async ({ page }) => {
	await openPicker(page);

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
	await openPicker(page);
	await expect(page.locator('.strip-cell[aria-current="true"]')).toHaveCount(1);
});

test("clicking a cell jumps there and closes the picker", async ({ page }) => {
	await openPicker(page);

	// Far enough in that it cannot be where the deck already was.
	await page.locator(".strip-cell").nth(5).click();

	// Closed, because the picker covers the notes the speaker just jumped to read.
	await expect(page.locator("dialog.picker")).toBeHidden();
	await expect(page.locator(".counter")).toContainText("6/");
	await expect(
		page.locator('.strip-cell[aria-current="true"]'),
	).toHaveAttribute("data-slide", "5");
});

test("g and / open the picker, Escape closes it", async ({ page }) => {
	await page.goto("/presenter");
	await expect(page.locator("dialog.picker")).toBeHidden();

	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toBeVisible();

	// The platform's own `Escape`, which is half the reason this is a `<dialog>`.
	await page.keyboard.press("Escape");
	await expect(page.locator("dialog.picker")).toBeHidden();
	await expect(page.locator(".strip-toggle")).toHaveAttribute(
		"aria-expanded",
		"false",
	);

	// `/` is unbound in the deck's keymap, so the picker may have it.
	await page.keyboard.press("/");
	await expect(page.locator("dialog.picker")).toBeVisible();
	await page.keyboard.press("Escape");
});

test("typing filters the grid down to the slides that match", async ({
	page,
	request,
}) => {
	const outline = await (await request.get("/api/outline")).json();
	const { word, index } = uniqueWord(outline.slides, "text");

	await openPicker(page);
	await page.fill(".strip-search", word);

	await expect(page.locator(".strip-cell:visible")).toHaveCount(1);
	await expect(page.locator(".strip-cell:visible")).toHaveAttribute(
		"data-slide",
		String(index),
	);
	await expect(page.locator(".strip-count")).toContainText("1 of");
	// The reason the slide matched, since it is not the title.
	await expect(page.locator(".strip-cell:visible mark")).toHaveText(
		new RegExp(word, "i"),
	);
});

test("a word only the speaker notes carry finds its slide", async ({
	page,
	request,
}) => {
	// The point of searching the notes at all: mid-talk, a speaker looking for a
	// slide often remembers what they meant to say about it rather than what it
	// shows.
	const outline = await (await request.get("/api/outline")).json();
	const { word, index } = uniqueWord(outline.slides, "notes");

	await openPicker(page);
	await page.fill(".strip-search", word);

	await expect(page.locator(".strip-cell:visible")).toHaveCount(1);
	await expect(page.locator(".strip-cell:visible")).toHaveAttribute(
		"data-slide",
		String(index),
	);
});

test("Enter jumps to the selection", async ({ page, request }) => {
	const outline = await (await request.get("/api/outline")).json();
	const { word, index } = uniqueWord(outline.slides, "text");

	await openPicker(page);
	await page.fill(".strip-search", word);
	// The only match is selected, so `Enter` is the whole jump.
	await expect(
		page.locator('.strip-cell[aria-selected="true"]'),
	).toHaveAttribute("data-slide", String(index));

	await page.keyboard.press("Enter");
	await expect(page.locator("dialog.picker")).toBeHidden();
	await expect(page.locator(".counter")).toContainText(`${index + 1}/`);
});

/** Which slide the picker's selection is on, as a deck index. */
const selected = (page: Page) =>
	page.locator('.strip-cell[aria-selected="true"]').getAttribute("data-slide");

test("the arrows move the selection instead of the deck", async ({ page }) => {
	// The deck's own keymap stands down while the search box has focus —
	// `typing_into_editable` reads the event's `composedPath`, so it sees an
	// input inside the presenter's shadow root.
	await openPicker(page);
	const counter = await page.locator(".counter").textContent();
	// Captured before and after: `show()` already selects the slide the deck is
	// on, so "exactly one cell is selected" holds before the press too and
	// asserting only that would pass on a picker whose arrows did nothing.
	const before = await selected(page);

	await page.keyboard.press("ArrowRight");
	await expect(page.locator('.strip-cell[aria-selected="true"]')).toHaveCount(
		1,
	);
	const after = await selected(page);
	expect(after).not.toBe(before);
	expect(Number(after)).toBe(Number(before) + 1);
	// The room did not move: the deck never saw the key.
	expect(await page.locator(".counter").textContent()).toBe(counter);

	// Back where it started, so the rest of the file is not at the mercy of
	// where this left the selection.
	await page.keyboard.press("ArrowLeft");
	expect(await selected(page)).toBe(before);

	// A whole row at a time, off the resolved `grid-template-columns` — the one
	// number in the picker that is read back out of the browser's layout rather
	// than computed.
	await page.keyboard.press("ArrowDown");
	const down = Number(await selected(page));
	expect(down).toBeGreaterThan(Number(before) + 1);
	await page.keyboard.press("ArrowUp");
	expect(await selected(page)).toBe(before);

	await page.keyboard.press("Escape");
});

test("Ctrl+K opens the picker too", async ({ page }) => {
	// The third opener, and the one whose modifier logic is inverted from the
	// other two: `k` opens only *with* a modifier, where `g` and `/` open only
	// without one.
	await page.goto("/presenter");
	await expect(page.locator("dialog.picker")).toBeHidden();

	await page.keyboard.press("ControlOrMeta+k");
	await expect(page.locator("dialog.picker")).toBeVisible();

	await page.keyboard.press("Escape");
	await expect(page.locator("dialog.picker")).toBeHidden();
});

test("Home and End belong to the query, not the grid", async ({ page }) => {
	// The caret's keys while the search box has focus: taking them would leave
	// no way to reach either end of a mistyped query.
	await openPicker(page);
	const before = await selected(page);

	await page.keyboard.press("Home");
	expect(await selected(page)).toBe(before);
	await page.keyboard.press("End");
	expect(await selected(page)).toBe(before);

	await page.keyboard.press("Escape");
});

test("a thumbnail that will not load leaves the cell identifiable", async ({
	page,
}) => {
	// `error` does not bubble from an `<img>`, so the delegated listener has to
	// run in the capture phase — get that wrong and the cell keeps the browser's
	// broken-image glyph for the length of the talk, saying nothing about which
	// slide it was. The readiness probe cannot catch this: it asks about slide 0
	// and speaks for the whole set.
	await openPicker(page);

	const cell = page.locator(".strip-cell").nth(3);
	await cell.locator("img").evaluate((img) => {
		(img as HTMLImageElement).src = "/overview/slide/99999?v=0";
	});

	await expect(cell).toHaveAttribute("data-shot", "failed");
	// Still a jump target, and still numbered.
	await expect(cell.locator(".strip-number")).toHaveText("4");

	await page.keyboard.press("Escape");
});

test("the picker does not register a client of its own", async ({
	page,
	request,
}) => {
	// The thumbnails are photographed against a private server, so opening the
	// picker must not add a client to the room — the same promise the shot page
	// makes, checked from the other end.
	await openPicker(page);
	const { clients } = await (await request.get("/api/clients")).json();
	expect(clients.length).toBe(1);
});

test("a live reload refreshes the picker's search corpus", async ({
	page,
	request,
}) => {
	// The one path nothing else covers: `set_talk` invalidates the thumbnails,
	// rebuilds the grid and re-probes on every `TalkChange`, and `fetch_outline`
	// runs again behind it. A picker that kept the old corpus would answer a
	// speaker's query from a deck that no longer exists — and it would do it
	// confidently, which is the failure worth a test that writes to disk.
	//
	// Last in the file, and restored in a `finally`: the whole suite is serial
	// against one server holding one deck.
	const slides =
		process.env.TOBOGGAN_TEST_DECK ?? "../examples/toboggan-guide/slides";
	const file = `${slides}/5_server/5-presenter.md`;
	const original = await readFile(file, "utf8");
	const planted = "zarbitraire";

	try {
		await openPicker(page);
		await page.fill(".strip-search", planted);
		await expect(page.locator(".strip-cell:visible")).toHaveCount(0);

		// Into the notes, so nothing about the slide's layout or the deck's
		// length changes — only what the deck *says*.
		await writeFile(file, `${original}\n\nAnd a note about ${planted}.\n`);

		// The corpus catches up: the watcher rebuilds, the socket says
		// `TalkChange`, and the client re-fetches `/api/outline`.
		await expect
			.poll(
				async () => {
					const outline = await (await request.get("/api/outline")).json();
					return outline.slides.some((slide: Entry) =>
						(slide.notes ?? "").includes(planted),
					);
				},
				{ timeout: 30_000 },
			)
			.toBe(true);

		// And the picker searches the new deck, not the one it opened on.
		await page.fill(".strip-search", "");
		await page.fill(".strip-search", planted);
		await expect(page.locator(".strip-cell:visible")).toHaveCount(1);
	} finally {
		await writeFile(file, original);
		// Let the restoring reload land before the next spec file opens a page
		// against this same server.
		await expect
			.poll(
				async () => {
					const outline = await (await request.get("/api/outline")).json();
					return outline.slides.some((slide: Entry) =>
						(slide.notes ?? "").includes(planted),
					);
				},
				{ timeout: 30_000 },
			)
			.toBe(false);
	}
});
