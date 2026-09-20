import type { Page } from "@playwright/test";
import { expect, test } from "@playwright/test";

/**
 * The bundled terminal font draws the icons a prompt actually uses.
 *
 * The terminal is a canvas, so a missing glyph is not a DOM fact: nothing in
 * the page says "tofu". What is observable is the painted pixels, so each
 * assertion here paints the icon and paints a codepoint mapped in no face, and
 * says they must differ — a face that does not cover the icon paints the two
 * identically, whatever the browser's missing-glyph box happens to look like.
 *
 * Comparing against plain `monospace` instead looks simpler and proves nothing:
 * the missing-glyph box is taken from the *first* available family, so the deck
 * stack and `monospace` differ on an unmapped codepoint too, and every
 * assertion would pass against the very subset that caused this spec.
 *
 * It is worth a spec of its own because the faces are generated (`mise
 * build:fonts`) and were wrong for a long time without anyone's build saying
 * so: they had been subset to the BMP, which silently drops the Material
 * Design set Nerd Fonts v3 moved to plane 15, and they carried icons in the
 * Regular face alone, which tofu'd every bold prompt segment.
 */

/** What `components/terminal/mod.rs` hands rioterm for `ctx.font`. */
const STACK =
	'"JetBrainsMono Nerd Font Mono", "JetBrainsMono Nerd Font Symbols", monospace';

/**
 * Codepoints from a real prompt (`starship` + `starship-jj`), one per way the
 * old subset failed.
 */
const ICONS = [
	{ codepoint: 0xf0035, what: "nf-md, plane 15 (starship [os.symbols] Macos)" },
	{ codepoint: 0xf15c6, what: "nf-md, plane 15 (starship-jj)" },
	{ codepoint: 0xf418, what: "nf-oct, BMP private use (branch)" },
	{ codepoint: 0xe0b0, what: "powerline separator" },
];

/**
 * Two codepoints past the end of the Material Design block, so mapped in no
 * face: one is every icon's reference, the other checks the reference itself.
 */
const UNMAPPED = [0xf1fff, 0xf1ffe].map((cp) => String.fromCodePoint(cp));

declare global {
	interface Window {
		tobogganFontsReady?: () => Promise<void>;
	}
}

/**
 * Paints each string at 32px and returns one hash per painted canvas.
 *
 * Batched into a single evaluate so a comparison is one round trip and both
 * sides are painted by the same context settings.
 */
const paint = (
	page: Page,
	texts: string[],
	weight: "" | "bold ",
	stack: string,
) =>
	page.evaluate(
		({ texts, weight, stack }) =>
			texts.map((text) => {
				const canvas = document.createElement("canvas");
				canvas.width = 64;
				canvas.height = 64;
				const context = canvas.getContext("2d");
				if (!context) {
					throw new Error("no 2d context");
				}
				context.textBaseline = "top";
				context.font = `${weight}32px ${stack}`;
				context.fillText(text, 4, 4);
				// FNV-1a over the pixels: the comparison only needs to know whether two
				// renders are the same image, and shipping 16 KB of RGBA back per probe
				// to learn that is a waste.
				const { data } = context.getImageData(0, 0, 64, 64);
				let hash = 0x811c9dc5;
				for (const byte of data) {
					hash = Math.imul(hash ^ byte, 0x01000193) >>> 0;
				}
				return hash;
			}),
		{ texts, weight, stack },
	);

test.beforeEach(async ({ page }) => {
	await page.goto("/run");
	// The faces are fetched lazily, by whoever opens the first terminal. Nothing
	// here opens one, so this test asks for them the same way the terminal does.
	await page.waitForFunction(
		() => typeof window.tobogganFontsReady === "function",
	);
	await page.evaluate(async () => {
		await window.tobogganFontsReady?.();
		await document.fonts.ready;
	});
});

test("two unmapped codepoints paint the same tofu", async ({ page }) => {
	// The control on the method: if these ever differ, the reference below stops
	// meaning "nothing was drawn" and every other test here goes green for free.
	const [first, second] = await paint(page, UNMAPPED, "", STACK);
	expect(first).toBe(second);
});

test("the bundled text face is what the deck draws with", async ({ page }) => {
	// Latin comes from the bundled face, not from the platform's monospace: if
	// this fails the stack never resolved, and the icons below prove nothing.
	const [bundled] = await paint(page, ["A"], "", STACK);
	const [fallback] = await paint(page, ["A"], "", "monospace");
	expect(bundled).not.toBe(fallback);
});

for (const { codepoint, what } of ICONS) {
	const name = `U+${codepoint.toString(16).toUpperCase()}`;
	for (const weight of ["", "bold "] as const) {
		const label = weight ? "bold" : "regular";
		test(`${name} renders at ${label} weight — ${what}`, async ({ page }) => {
			const icon = String.fromCodePoint(codepoint);
			const [drawn, tofu] = await paint(
				page,
				[icon, UNMAPPED[0]],
				weight,
				STACK,
			);
			expect(drawn).not.toBe(tofu);
		});
	}
}
