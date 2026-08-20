import { defineConfig, devices } from "@playwright/test";

/**
 * Smoke tests for a real server serving a real deck.
 *
 * The embedded web client is compiled Rust behind a shadow root, so the only
 * thing that can tell us the bundle actually works is a browser. The Rust gate
 * stubs `toboggan-web/dist` and therefore proves nothing about it.
 */
const port = Number(process.env.TOBOGGAN_TEST_PORT ?? 8137);
const deck =
	process.env.TOBOGGAN_TEST_DECK ?? "../examples/toboggan-guide/slides";

/**
 * A prebuilt binary when CI hands us one, `cargo run` otherwise.
 *
 * CI already builds the binary to run the other deck checks, and this repo sets
 * a `[build] target` so the debug binary is not where a naive path would look —
 * so the path is passed in rather than guessed.
 */
const binary = process.env.TOBOGGAN_BIN;
const command = binary
	? `${binary} -p ${deck} --host 127.0.0.1 --port ${port}`
	: `cargo run --manifest-path ../Cargo.toml -p toboggan -- -p ${deck} --host 127.0.0.1 --port ${port}`;

export default defineConfig({
	testDir: "./tests",
	// One server, one presentation state: `POST /api/command` and every keystroke
	// move the deck for *every* test in the run. Specs that navigate therefore
	// cannot share a run with each other, let alone with themselves — so the
	// suite is serial across files, not just within them.
	fullyParallel: false,
	workers: 1,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 1 : 0,
	reporter: process.env.CI ? "github" : "list",
	use: {
		baseURL: `http://127.0.0.1:${port}`,
		trace: "on-first-retry",
	},
	projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
	webServer: {
		command,
		url: `http://127.0.0.1:${port}/health`,
		reuseExistingServer: !process.env.CI,
		// A cold `cargo run` builds the whole workspace first.
		timeout: 300_000,
		stdout: "pipe",
		stderr: "pipe",
	},
});
