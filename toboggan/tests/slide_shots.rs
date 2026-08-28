//! `toboggan thumbnails --thumbnail-renderer browser`, against a real browser.
//!
//! Everything below the CLI is covered elsewhere — the shot page has its own
//! Playwright suite, the browser detection has unit tests — and none of it
//! crosses the process boundary this test is about: launching a browser,
//! serving the deck to it privately, and getting one PNG per slide back out.
//!
//! The failure this exists to catch is the quiet one. A driver that photographs
//! `about:blank`, or the same slide twenty times, writes exactly as many files
//! of exactly the right name and exits 0.

// As the `#[cfg(test)]` modules in `src/` do: in a test, a failed `expect` with
// a message is the diagnostic.
#![allow(clippy::expect_used, clippy::print_stderr, clippy::indexing_slicing)]

use std::path::Path;
use std::process::Command;

/// A deck whose slides differ from each other, and whose second slide is one
/// the Typst renderer could not draw: `<style>` and raw HTML are dropped by
/// `output/typst.rs`, so this is the case the browser renderer exists for.
fn write_deck(slides: &Path) {
    std::fs::create_dir_all(slides).expect("create slides dir");
    std::fs::write(slides.join("_cover.md"), "# Shot Deck\n").expect("write cover");
    std::fs::write(
        slides.join("1-plain.md"),
        "+++\ntitle = \"Plain\"\n+++\n\n# Plain\n\nA line of text.\n",
    )
    .expect("write slide 1");
    std::fs::write(
        slides.join("2-styled.md"),
        "+++\ntitle = \"Styled\"\n+++\n\n<style>\n  h1 { color: #c0392b; }\n</style>\n\n\
         # Styled\n\n<div class=\"box\">Raw HTML, which Typst drops.</div>\n",
    )
    .expect("write slide 2");
}

/// Skips when no browser is installed — unless this is CI, where absent means
/// the workflow stopped installing one and this test quietly stopped testing.
///
/// The same guard, and for the same reason, as `overflow_check`'s `typst`
/// check: a test that can silently become a no-op is the failure mode the
/// feature under test exists to prevent.
fn browser_available() -> bool {
    let found = toboggan_server::find_browser(None).is_some();
    assert!(
        found || std::env::var_os("CI").is_none(),
        "no browser is installed but CI is set: the shot renderer would go untested"
    );
    if !found {
        eprintln!("skipping: no Chrome, Chromium or Edge on this machine");
    }
    found
}

/// A PNG's declared size, read straight out of the IHDR chunk.
///
/// Eight bytes of signature, then the chunk length and type, then width and
/// height as big-endian `u32`s. Cheaper than an image decoder, and the only
/// thing that has to be true of the bytes for the assertion to mean anything.
fn png_size(bytes: &[u8]) -> (u32, u32) {
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "not a PNG: {:?}",
        &bytes[..bytes.len().min(8)]
    );
    let read = |at: usize| {
        u32::from_be_bytes(
            bytes[at..at + 4]
                .try_into()
                .expect("four bytes for a dimension"),
        )
    };
    (read(16), read(20))
}

#[test]
fn the_browser_renderer_photographs_every_slide() {
    if !browser_available() {
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let slides = dir.path().join("slides");
    write_deck(&slides);
    let out = dir.path().join("overview");

    let output = Command::new(env!("CARGO_BIN_EXE_toboggan"))
        .arg("thumbnails")
        .arg("-p")
        .arg(&slides)
        .arg("-o")
        .arg(&out)
        .arg("--thumbnail-renderer")
        .arg("browser")
        .env_remove("RUST_LOG")
        .output()
        .expect("run toboggan thumbnails");
    assert!(
        output.status.success(),
        "thumbnails failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(out.join("overview.html").is_file(), "no overview page");

    // Three slides: the cover and the two written above.
    let shots = (0..3)
        .map(|index| {
            let path = out.join(format!("thumb-{index:04}.png"));
            std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
        })
        .collect::<Vec<_>>();

    for (index, png) in shots.iter().enumerate() {
        // Laid out at the projector's 1280x720 and captured at half of it.
        assert_eq!(png_size(png), (640, 360), "slide {index} is the wrong size");
        // A blank frame of one flat colour compresses to a couple of kilobytes.
        // Anything with a slide on it is far larger, and this is the assertion
        // that fails if the driver photographs `about:blank`.
        assert!(
            png.len() > 4_000,
            "slide {index} is {} bytes, which is about what an empty frame weighs",
            png.len()
        );
    }

    // Three different slides, three different pictures. Without this a driver
    // that never navigates — or that shoots before the new slide has painted —
    // passes everything above.
    assert_ne!(shots[0], shots[1], "the cover and slide 1 look identical");
    assert_ne!(shots[1], shots[2], "slides 1 and 2 look identical");
}

/// A `hidden_in = ["web"]` slide must not shift the thumbnails after it.
///
/// The overview is an authoring view: `thumb-NNNN.png` is slide N of the deck
/// *as authored*, and a web-hidden slide gets a card of its own with a badge on
/// it. But the server that the shots are taken against drops those slides and
/// renumbers what is left, so photographing by full-deck index read the *next*
/// slide for every index past the hidden one — filed under the right name, with
/// nothing to say it was wrong. The guide, with one hidden slide among 44, ran
/// off the end and failed loudly; a deck whose hidden slide is nearer the middle
/// would simply have published a shifted overview.
///
/// Asserted by shooting the deck twice — once with the marker, once without —
/// and demanding the same pictures. Rendering is deterministic here (fixed
/// viewport, animations neutralised), and nothing else about the two decks
/// differs, so a byte difference is a shift.
#[test]
fn a_web_hidden_slide_does_not_shift_the_thumbnails_after_it() {
    if !browser_available() {
        return;
    }

    let shoot = |hide_the_middle: bool| {
        let dir = tempfile::tempdir().expect("temp dir");
        let slides = dir.path().join("slides");
        write_deck(&slides);
        if hide_the_middle {
            // Slide 1 of three, so there is one before it and one after it.
            let path = slides.join("1-plain.md");
            let source = std::fs::read_to_string(&path).expect("read slide 1");
            std::fs::write(
                &path,
                source.replace(
                    "title = \"Plain\"",
                    "title = \"Plain\"\nhidden_in = [\"web\"]",
                ),
            )
            .expect("hide slide 1");
        }

        let out = dir.path().join("overview");
        let output = Command::new(env!("CARGO_BIN_EXE_toboggan"))
            .arg("thumbnails")
            .arg("-p")
            .arg(&slides)
            .arg("-o")
            .arg(&out)
            .arg("--thumbnail-renderer")
            .arg("browser")
            .env_remove("RUST_LOG")
            .output()
            .expect("run toboggan thumbnails");
        assert!(
            output.status.success(),
            "thumbnails failed (hidden={hide_the_middle}): {}",
            String::from_utf8_lossy(&output.stderr)
        );

        (0..3)
            .map(|index| {
                let path = out.join(format!("thumb-{index:04}.png"));
                std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
            })
            .collect::<Vec<_>>()
    };

    let hidden = shoot(true);
    let shown = shoot(false);

    for index in 0..3 {
        assert_eq!(
            hidden[index], shown[index],
            "thumbnail {index} changed when a slide was hidden from the web"
        );
    }
}

/// `--thumbnail-renderer browser` must fail rather than fall back.
///
/// The distinction `auto` exists to blur and CI exists to keep sharp: a
/// published site that silently switched to the approximate renderer is worse
/// than a build that stopped and said why.
#[test]
fn an_explicit_browser_that_is_not_there_fails_rather_than_falling_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let slides = dir.path().join("slides");
    write_deck(&slides);
    let out = dir.path().join("overview");

    let output = Command::new(env!("CARGO_BIN_EXE_toboggan"))
        .arg("thumbnails")
        .arg("-p")
        .arg(&slides)
        .arg("-o")
        .arg(&out)
        .arg("--thumbnail-renderer")
        .arg("browser")
        .arg("--browser")
        .arg(dir.path().join("no-such-browser"))
        .env_remove("RUST_LOG")
        .output()
        .expect("run toboggan thumbnails");

    assert!(!output.status.success(), "a missing browser exited 0");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("browser"),
        "the error does not mention the browser: {stderr}"
    );
    assert!(
        !out.join("overview.html").is_file(),
        "an overview was published anyway"
    );
}
