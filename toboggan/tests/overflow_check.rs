//! The overflow check, against a real `typst`.
//!
//! Every other test of this feature stops at the process boundary: `spans_from`
//! is well covered, and nothing exercises the query string, the `eval`
//! invocation, or the marker label the two crates have to agree on. A typo in
//! any of those produces an empty result, which the command reports on stderr
//! and still exits 0 for — so the feature can be entirely dead with the whole
//! suite green. This is the test that notices.

// As the `#[cfg(test)]` modules in `src/` do: in a test, a failed `expect` with
// a message is the diagnostic.
#![allow(clippy::expect_used, clippy::print_stderr)]

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

/// A slide with far more lines than one page can hold.
const OVERFLOWING: &str = "+++
title = \"Too Much\"
+++

# Too Much

";

/// A slide that comfortably fits.
const FITS: &str = "+++
title = \"Just Right\"
+++

# Just Right

One short line.
";

/// Skips when `typst` is absent — unless this is CI, where absent means the
/// workflow stopped installing it and this test quietly stopped testing.
///
/// A test that can silently become a no-op is the same failure mode the feature
/// under test exists to prevent, so it is worth the four lines.
fn typst_available() -> bool {
    let found = Command::new("typst")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success());
    assert!(
        found || std::env::var_os("CI").is_none(),
        "`typst` is not on PATH but CI is set: the overflow check would go untested"
    );
    if !found {
        eprintln!("skipping: `typst` is not on PATH");
    }
    found
}

/// Writes a deck whose second slide cannot fit on one page.
fn write_deck(slides: &Path) {
    std::fs::create_dir_all(slides).expect("create slides dir");
    std::fs::write(slides.join("_cover.md"), "# Overflow Deck\n").expect("write cover");
    std::fs::write(slides.join("1-fits.md"), FITS).expect("write fitting slide");

    let mut too_much = String::from(OVERFLOWING);
    for line in 0..60 {
        let _ = writeln!(too_much, "Line {line} of a slide that will not fit.\n");
    }
    std::fs::write(slides.join("2-overflows.md"), &too_much).expect("write overflowing slide");
}

fn run_pdf(slides: &Path, out: &Path, extra: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_toboggan"))
        .arg("pdf")
        .arg("-p")
        .arg(slides)
        .arg("-o")
        .arg(out)
        .args(extra)
        .env_remove("RUST_LOG")
        .output()
        .expect("run toboggan pdf")
}

#[test]
fn the_overflow_check_names_the_slide_that_spilled() {
    if !typst_available() {
        return;
    }
    let deck = tempfile::tempdir().expect("temp deck");
    let slides = deck.path().join("slides");
    write_deck(&slides);
    let out = deck.path().join("deck.pdf");

    let result = run_pdf(&slides, &out, &[]);
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    assert!(
        result.status.success(),
        "a deck that does not fit is still a PDF\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        out.exists(),
        "the PDF is the deliverable, and it was written"
    );

    // The whole point: the check ran, found the markers, and said which slide.
    assert!(
        stderr.is_empty() || !stderr.contains("could not check"),
        "the check itself must not fail: {stderr}"
    );
    assert!(
        stdout.contains("2-overflows.md"),
        "the slide that spilled is named\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !stdout.contains("1-fits.md"),
        "the slide that fits is not\nstdout: {stdout}"
    );
}

#[test]
fn no_overflow_check_skips_the_report() {
    if !typst_available() {
        return;
    }
    let deck = tempfile::tempdir().expect("temp deck");
    let slides = deck.path().join("slides");
    write_deck(&slides);
    let out = deck.path().join("deck.pdf");

    let result = run_pdf(&slides, &out, &["--no-overflow-check"]);
    let stdout = String::from_utf8_lossy(&result.stdout);

    assert!(result.status.success(), "the PDF is still written");
    assert!(out.exists(), "and still on disk");
    assert!(
        !stdout.contains("2-overflows.md"),
        "the flag skips the pass, so nothing is reported\nstdout: {stdout}"
    );
}

#[test]
fn a_deck_that_fits_reports_no_overflow() {
    if !typst_available() {
        return;
    }
    let deck = tempfile::tempdir().expect("temp deck");
    let slides = deck.path().join("slides");
    std::fs::create_dir_all(&slides).expect("create slides dir");
    std::fs::write(slides.join("_cover.md"), "# Small Deck\n").expect("write cover");
    std::fs::write(slides.join("1-fits.md"), FITS).expect("write fitting slide");
    let out = deck.path().join("deck.pdf");

    let result = run_pdf(&slides, &out, &[]);
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    assert!(result.status.success(), "stderr: {stderr}");
    assert!(
        !stdout.contains("do not fit"),
        "nothing overflowed, so nothing is reported\nstdout: {stdout}"
    );
    // The marker count still has to line up, or the run would have said so —
    // this is what distinguishes "the check passed" from "the check never ran".
    assert!(
        !stderr.contains("could not check"),
        "the check ran and accounted for every slide: {stderr}"
    );
}
