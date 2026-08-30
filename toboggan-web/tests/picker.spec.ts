import { expect, type Page, test } from "@playwright/test";

/**
 * The slide picker, on both pages that mount one: every slide at once, a search
 * box over them, and a way to jump.
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

/**
 * Opens `/presenter` and waits until its keyboard is actually live.
 *
 * `toBeHidden()` is no barrier here — it passes for an element that does not
 * exist yet — so a test that navigates and immediately types races the wasm
 * client's `keydown` listener and loses the key. `data-ready` is set once every
 * listener is armed.
 */
async function openPresenter(page: Page) {
	await page.goto("/presenter");
	await expect(page.locator(".layout")).toHaveAttribute("data-ready", "true", {
		timeout: 30_000,
	});
}

/** Opens the picker, and waits for the thumbnails behind it. */
async function openPicker(page: Page) {
	await openPresenter(page);
	await page.locator(".strip-toggle").click();
	await expect(page.locator("dialog.picker")).toHaveAttribute(
		"data-previews",
		"ready",
		// The whole deck is photographed before the first cell can be filled.
		{ timeout: 60_000 },
	);
}

test("the picker is behind a shadow root of its own", async ({ page }) => {
	// The whole reason this is a component rather than part of the presenter's
	// markup: the top layer is not a style boundary. Mounted straight into the
	// document that shows a deck, the dialog would be styled by whatever the
	// author wrote in `_head.html` — the same way that CSS once restyled the
	// speaker's chrome. Nothing else here can see the difference, and nothing
	// else here would fail if it were lost.
	await openPresenter(page);

	const placement = await page.evaluate(() => {
		const shell = document.querySelector("main")?.shadowRoot;
		const hosts = [...(shell?.children ?? [])].filter((el) => el.shadowRoot);
		return {
			// Not in the presenter's own tree, where it used to be written...
			inTheShell: shell?.querySelector("dialog.picker") !== null,
			// ...but one boundary further down, in the component's.
			inItsOwn: hosts.some((el) =>
				el.shadowRoot?.querySelector("dialog.picker"),
			),
		};
	});

	expect(placement).toEqual({ inTheShell: false, inItsOwn: true });
});

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

	// Through a locator, not `document.querySelectorAll`: the cells are inside
	// the picker's shadow root, which `querySelectorAll` does not enter. Asking
	// the document directly returns an empty list on a page that is working and
	// an empty list on a page that is not — so the assertion below passed for
	// the whole life of this test without ever seeing an `<img>`. Playwright's
	// locators pierce shadow roots; that is the entire difference.
	const images = page.locator(".strip-cell img");
	expect(await images.count()).toBeGreaterThan(0);

	// `naturalWidth` rather than the `src` attribute: a `503` while the deck is
	// still being photographed sets the attribute perfectly well and renders a
	// broken image, which is the failure this whole probe-and-retry exists for.
	const undecoded = await images.evaluateAll(
		(found) =>
			found.filter((img) => !(img as HTMLImageElement).naturalWidth).length,
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
	await openPresenter(page);
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
	// The deck's own keymap stands down for as long as the dialog is open: it
	// holds a claim on the keyboard, the way a terminal on a slide does. Focus
	// is not what answers this — see "an open picker keeps the deck's keys when
	// its box loses focus", which is the case focus alone got wrong.
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
	await openPresenter(page);
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

/**
 * The same picker, on the deck page.
 *
 * `/run` is where a speaker presenting off one screen actually is, and until the
 * picker became a component the only way to reach slide 31 from there was to
 * remember the number. What has to hold is that it is the *same* picker — same
 * keys, same jump — and that the two other pages `/run` serves get none of it: a
 * picker on the shot page would be photographed into a thumbnail.
 */
async function openDeck(page: Page) {
	await page.goto("/run");
	// The slide says the client is up; the cells say the picker has heard what
	// shape the deck is, which arrives a `/api/talk` round trip later.
	await page.waitForSelector(".toboggan-slide", { timeout: 30_000 });
	await expect(page.locator(".strip-cell").first()).toBeAttached({
		timeout: 30_000,
	});
}

/** Opens the deck page's picker, and waits for the thumbnails behind it. */
async function openDeckPicker(page: Page) {
	await openDeck(page);
	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toBeVisible();
	// The same wait `openPicker` makes on the presenter, and not a formality:
	// the deck page reaches the probe by a different call altogether — its
	// picker owns its `Thumbnails` and starts them from `reload`/`sync`, where
	// the presenter drives the set it shares. A deck picker stuck for ever on
	// "Rendering slide previews…" passed every test in this file.
	await expect(page.locator("dialog.picker")).toHaveAttribute(
		"data-previews",
		"ready",
		{ timeout: 60_000 },
	);
}

/** Which slide the deck is on, 1-based — what its own CSS counts by. */
const deckIsOn = (page: Page) =>
	page.evaluate(() =>
		document.querySelector("main")?.style.getPropertyValue("--current-slide"),
	);

test("the deck page mounts the same picker", async ({ page, request }) => {
	const { slides } = await (await request.get("/api/slides")).json();

	await openDeck(page);
	await expect(page.locator("dialog.picker")).toBeHidden();

	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toBeVisible();
	await expect(page.locator(".strip-cell")).toHaveCount(slides.length);

	await page.keyboard.press("Escape");
	await expect(page.locator("dialog.picker")).toBeHidden();
});

test("a jump from the deck's own picker moves the deck", async ({
	page,
	request,
}) => {
	const outline = await (await request.get("/api/outline")).json();
	const { word, index } = uniqueWord(outline.slides, "text");

	await openDeck(page);
	await page.keyboard.press("g");
	await page.fill(".strip-search", word);
	// The only match, so `Enter` is the whole jump.
	await expect(
		page.locator('.strip-cell[aria-selected="true"]'),
	).toHaveAttribute("data-slide", String(index));

	await page.keyboard.press("Enter");
	await expect(page.locator("dialog.picker")).toBeHidden();
	// Not the picker's own idea of where it is: the deck moved, which on this
	// page means the slide the room is looking at moved.
	await expect.poll(() => deckIsOn(page)).toBe(String(index + 1));
});

test("the deck page's picker is behind a shadow root of its own", async ({
	page,
}) => {
	// Asserted on `/run` and not only on `/presenter`, because `/run` is the
	// page it matters on: this is the document a deck's `_head.html` is injected
	// into, and the top layer is not a style boundary. Playwright's locators
	// pierce shadow roots, so every other deck test in this file would go on
	// passing if the dialog were mounted into the light DOM tomorrow.
	//
	// Structural rather than a computed-style check: the guide's `_head.html`
	// sets custom properties on `:root`, and those cross a shadow boundary by
	// design — so a colour test would prove nothing either way.
	await openDeck(page);

	const placement = await page.evaluate(() => {
		const hosts = [...document.body.children].filter((el) => el.shadowRoot);
		return {
			// Not in the deck's own document, where `_head.html` could reach it...
			inTheDocument: document.querySelector("dialog.picker") !== null,
			// ...but inside a shadow root of its own, under <body>.
			inItsOwn: hosts.some((el) => el.shadowRoot?.querySelector("dialog.picker")),
		};
	});

	expect(placement).toEqual({ inTheDocument: false, inItsOwn: true });
});

test("every one of the deck picker's cells shows a picture", async ({
	page,
}) => {
	await openDeckPicker(page);

	const images = page.locator(".strip-cell img");
	expect(await images.count()).toBeGreaterThan(0);
	const undecoded = await images.evaluateAll(
		(found) =>
			found.filter((img) => !(img as HTMLImageElement).naturalWidth).length,
	);
	expect(undecoded).toBe(0);
});

test("all three openers work on the deck page", async ({ page }) => {
	// `g` alone was covered. `/` is Chrome's own quick-find and `Ctrl+K` its
	// omnibox, so both depend on the `preventDefault` in the picker's `window`
	// listener, and `Ctrl+K` is the one opener whose modifier logic is inverted
	// from the other two. On the presenter a dead opener still leaves the `▦`
	// button; here there is nothing else to press.
	await openDeck(page);

	for (const key of ["g", "/", "Control+k"]) {
		await page.keyboard.press(key);
		await expect(page.locator("dialog.picker")).toBeVisible();
		await page.keyboard.press("Escape");
		await expect(page.locator("dialog.picker")).toBeHidden();
	}
});

test("typing a query does not reach the deck", async ({ page }) => {
	// `page.fill()` assigns `.value` and dispatches one `input` event — it emits
	// no `keydown` at all, so every other test in this file typed a query
	// without the deck's `window` listener ever seeing a character. This is the
	// one that does, and it is the only cover `typing_into_editable` has.
	//
	// On `/run` the stakes are real: `g` and `/` open the picker, and the digits
	// are bound to typing a slide number at the *room's* screen.
	await openDeck(page);
	const on = await deckIsOn(page);

	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toBeVisible();

	const query = "g/3 log";
	await page.locator(".strip-search").pressSequentially(query, { delay: 20 });

	// Still open: the `g` and the `/` were characters, not openers.
	await expect(page.locator("dialog.picker")).toBeVisible();
	// Every character landed in the box, and only there.
	await expect(page.locator(".strip-search")).toHaveValue(query);
	// The digit did not start a goto on the projected screen.
	await expect(page.locator("#toboggan-goto")).toHaveCount(0);
	expect(await deckIsOn(page)).toBe(on);

	await page.keyboard.press("Escape");
});

test("a click on a cell jumps the deck", async ({ page }) => {
	// The other half of `install_jump`: `Enter` is covered above, but a click is
	// what the mouse-holding half of the deck page does, and it arrives through
	// a delegated listener that has to find its cell inside the shadow root.
	await openDeckPicker(page);

	const target = page.locator('.strip-cell[data-slide="4"]');
	await target.click();
	await expect(page.locator("dialog.picker")).toBeHidden();
	await expect.poll(() => deckIsOn(page)).toBe("5");
});

test("the mirror pane mounts no picker", async ({ page }) => {
	// The pair of "the shot page mounts no picker", and the one the presenter
	// view depends on: `/run?mirror=current` is the iframe inside the speaker's
	// own layout, so a picker there would open *inside* the current-slide pane
	// and eat the keys meant for the presenter. Today `main.ts` returns before
	// `start_app` ever runs — one `return` in TypeScript, guarded by nothing
	// else.
	await page.goto("/run?mirror=current");
	await page.waitForSelector(".toboggan-slide", { timeout: 30_000 });

	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toHaveCount(0);
});

test("an open picker keeps the deck's keys when its box loses focus", async ({
	page,
}) => {
	// The picker's keys hang off the dialog; the deck's hang off `window`; the
	// event reaches both. What used to stand the deck down was
	// `typing_into_editable` alone, which answers for a text field and nothing
	// else — so a click on the dialog's own hint bar, or one `Tab` to the ✕,
	// moved focus off the search box and left `ArrowRight` advancing the room's
	// slide from behind an open dialog. On `/run` that slide is the one the
	// audience is watching.
	await openDeck(page);
	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toBeVisible();

	const on = await deckIsOn(page);
	const before = await selected(page);

	// Focus onto something inside the dialog that is not an input.
	await page.locator(".strip-hint").click();
	await page.keyboard.press("ArrowRight");
	expect(Number(await selected(page))).toBe(Number(before) + 1);
	// The picker moved. The room did not.
	expect(await deckIsOn(page)).toBe(on);

	// And a key the picker's `match` does not name: a digit falls past every arm
	// of it to the deck's own listener, so nothing written inside the picker's
	// handler can protect it — only a claim on the keyboard can. The badge is
	// what the room would see.
	await page.keyboard.press("3");
	await page.waitForTimeout(400);
	await expect(page.locator("#toboggan-goto")).toHaveCount(0);
	expect(await deckIsOn(page)).toBe(on);

	await page.keyboard.press("Escape");
	await expect(page.locator("dialog.picker")).toBeHidden();
	// Handed straight back, so closing the picker does not cost the deck its
	// keys for the rest of the talk.
	await page.keyboard.press("ArrowRight");
	await expect.poll(() => deckIsOn(page)).toBe(String(Number(on) + 1));
	await page.keyboard.press("ArrowLeft");
	await expect.poll(() => deckIsOn(page)).toBe(on);
});

test("a failed outline says so instead of matching everything", async ({
	page,
}) => {
	// An empty corpus filters nothing, so every cell goes on matching every
	// query. The count is the only thing that can tell a speaker the difference
	// between "your word is on all forty slides" and "I never looked" — and the
	// first of those is a specific, confident, wrong answer to a question the
	// picker never asked.
	await page.route("**/api/outline", (route) =>
		route.fulfill({ status: 500, body: "no outline for you" }),
	);

	await openDeck(page);
	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toHaveAttribute(
		"data-search",
		"unavailable",
	);

	const cells = await page.locator(".strip-cell").count();
	await page.fill(".strip-search", "wordthedeckhasnever");
	// The grid still shows the deck and still jumps: losing the search is the
	// documented cost of a failed `/api/outline`, and a picker that showed
	// nothing would be worse than one that shows everything.
	await expect(page.locator(".strip-cell:not([hidden])")).toHaveCount(cells);
	// It just does not call that a result.
	await expect(page.locator(".strip-count")).toContainText(
		"search unavailable",
	);
	await expect(page.locator(".strip-count")).not.toContainText(
		`${cells} of ${cells}`,
	);

	await page.keyboard.press("Escape");
});

test("a terminal keeps the keys that would open the picker", async ({
	page,
}) => {
	// Only the deck page can go wrong this way — the presenter view has no
	// terminal. `g` and `/` are ordinary characters at a shell prompt, and a
	// path alone is full of the second one. The picker installs a `keydown`
	// listener of its own on `window`, separate from the deck's keymap: both
	// stand down through the same guard, and nothing else here proves the
	// picker's half of it.
	await openDeck(page);

	await page.keyboard.press("Backquote");
	await expect(page.locator(".toboggan-quake-terminal")).toHaveClass(/open/);
	await expect(
		page.locator(".toboggan-quake-inner .terminal-window"),
	).toHaveClass(/terminal-has-keys/);

	await page.keyboard.press("g");
	await page.keyboard.press("/");
	// "Nothing happened" is given time to happen anyway.
	await page.waitForTimeout(400);
	await expect(page.locator("dialog.picker")).toBeHidden();

	// Handed back, so the same two keys mean the picker again.
	await page.keyboard.press("Backquote");
	await expect(page.locator(".toboggan-quake-terminal")).not.toHaveClass(
		/open/,
	);
	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toBeVisible();
	await page.keyboard.press("Escape");
	await expect(page.locator("dialog.picker")).toBeHidden();
});

test("the shot page mounts no picker", async ({ page }) => {
	// Every thumbnail is a photograph of `/run`, so anything `/run` mounts can
	// end up inside one. The shot page is a separate entry point for exactly
	// this kind of reason, and this is what keeps it one.
	await page.goto("/run?shot=1");
	await expect(page.locator("html")).toHaveAttribute(
		"data-toboggan-shot",
		"ready",
		{ timeout: 60_000 },
	);

	await page.keyboard.press("g");
	await expect(page.locator("dialog.picker")).toHaveCount(0);
});

// Not covered here: the picker on a client that may not drive the deck. Roles
// are assigned by peer address (`auth.rs::role_for`) and every connection this
// suite makes is loopback, so a browser here is always the presenter — there is
// no way to reach `data-role="audience"` short of serving the deck to a second
// machine or setting a token. What that path has to do is refuse the jump
// *without* closing, so the standing hint stays readable rather than a toast
// arriving behind a dialog that is already shutting.
//
// Not covered here: the live-reload path (`set_talk` invalidating thumbnails,
// rebuilding the grid and re-fetching `/api/outline`). A test for it has to edit
// a slide on disk, and this whole suite is serial against one server holding one
// deck — so the reload it triggers, and the re-photographing behind it, reached
// specs that ran afterwards and made them fail on timing rather than on
// behaviour. It belongs against a server and a deck copy of its own.
