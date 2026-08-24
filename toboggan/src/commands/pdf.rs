use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use toboggan_cli::OutputFormat;

/// Builds a PDF from a folder by rendering Typst, then shelling out to `typst`.
///
/// # Errors
/// Returns an error if parsing fails, the `.typ` cannot be written, the `typst`
/// binary is missing, or compilation fails. A failing *overflow check* is not
/// one of them: the PDF is already written by then, so it is reported on stderr
/// and the command still succeeds.
#[allow(clippy::print_stdout, clippy::print_stderr)]
pub(crate) fn build_pdf(
    input: &Path,
    mut settings: toboggan_cli::Settings,
    output: Option<PathBuf>,
    check_overflow: bool,
) -> anyhow::Result<()> {
    super::ensure_typst()?;

    let deck = super::deck::resolve_deck(input);
    settings.input = Some(deck.slides.clone());
    let output = output.unwrap_or_else(|| default_pdf_path(&deck.slides));

    let talk = super::deck::build_talk(&deck.slides, &settings)?;
    let typst_source = toboggan_cli::output::serialize_talk(
        &talk,
        OutputFormat::Typst,
        "",
        &toboggan_cli::mermaid_renderer(&settings)?,
    )
    .map_err(|err| anyhow::anyhow!("{err}"))?;

    // Two things have to line up for a slide's `#image("../public/logo.png")` to
    // resolve: the intermediate `.typ` must sit where the slides do (typst
    // resolves relative paths against the *file*), and the project root must be
    // the deck (typst refuses any path that escapes it). Compiling a temp file
    // from elsewhere failed every such slide with "would escape the project root".
    let root = deck.root();
    let typ_path = deck.slides.join(".toboggan-pdf.typ");
    std::fs::write(&typ_path, &typst_source)
        .map_err(|err| anyhow::anyhow!("writing {}: {err}", typ_path.display()))?;

    let status = Command::new("typst")
        .arg("compile")
        .arg("--root")
        .arg(&root)
        .arg(&typ_path)
        .arg(&output)
        .status()
        .map_err(|err| {
            anyhow::anyhow!("could not run `typst` (is it installed and on PATH?): {err}")
        })?;

    // The overflow check reads the same `.typ`, so it has to run before the
    // scratch file goes — and only once there is a PDF worth checking.
    let spans = (status.success() && check_overflow)
        .then(|| slide_spans(&root, &typ_path))
        .transpose();

    // Best-effort: the PDF is the deliverable, and leaving the scratch file
    // behind on success was a reported annoyance.
    if let Err(err) = std::fs::remove_file(&typ_path) {
        tracing::debug!("could not remove {}: {err}", typ_path.display());
    }

    if !status.success() {
        anyhow::bail!("`typst compile` failed with status {status}");
    }

    println!("✅ Wrote {}", output.display());
    match spans {
        // A slide that does not fit is still a PDF: say so and exit 0.
        Ok(Some(spans)) => report_overflow(&spans),
        Ok(None) => {}
        // Printed for the same reason `report_overflow` prints: the default
        // filter drops warnings, so a `tracing::warn!` here left an unqualified
        // ✅ as the last word and told nobody the check had not run. A check
        // that fails silently is the thing this whole pass exists to prevent.
        Err(err) => eprintln!("⚠️  could not check the deck for slide overflow: {err:#}"),
    }
    Ok(())
}

/// The Typst expression that reports where every slide starts and ends.
///
/// The markers `toboggan-cli` emits are `#metadata`, so they render nothing and
/// shift nothing; asking typst where they landed is the only way to learn that a
/// slide spilled onto a second page.
const SLIDE_PAGES_QUERY: &str = "query(<toboggan-slide>).map(m => \
     (slide: m.value.slide, at: m.value.at, page: m.location().page()))";

/// One marker as typst reports it.
#[derive(Debug, Deserialize)]
struct SlideMarker {
    slide: String,
    at: String,
    page: usize,
}

/// The pages one slide occupies in the compiled PDF, first and last.
#[derive(Debug, PartialEq, Eq)]
struct SlideSpan {
    slide: String,
    start: usize,
    end: usize,
}

impl SlideSpan {
    const fn pages(&self) -> usize {
        // Both ends are inclusive, and `end` cannot precede `start`: the
        // markers come back in document order.
        self.end.saturating_sub(self.start) + 1
    }
}

/// Asks typst which pages each slide occupies.
fn slide_spans(root: &Path, typ_path: &Path) -> anyhow::Result<Vec<SlideSpan>> {
    let output = Command::new("typst")
        .arg("eval")
        .arg("--root")
        .arg(root)
        .arg("--in")
        .arg(typ_path)
        .arg(SLIDE_PAGES_QUERY)
        .output()
        .map_err(|err| anyhow::anyhow!("could not run `typst eval`: {err}"))?;

    if !output.status.success() {
        anyhow::bail!(
            "`typst eval` failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let markers = serde_json::from_slice::<Vec<SlideMarker>>(&output.stdout)
        .map_err(|err| anyhow::anyhow!("could not read the page markers: {err}"))?;
    spans_from(&markers)
}

/// Pairs the markers up, one span per slide.
///
/// They arrive in document order, so each slide is a start immediately followed
/// by its own end. Anything else means the two sides have drifted apart, and
/// guessing a pairing would report overflow on the wrong slide.
fn spans_from(markers: &[SlideMarker]) -> anyhow::Result<Vec<SlideSpan>> {
    markers
        .chunks(2)
        .map(|pair| match pair {
            [start, end] if start.at == "start" && end.at == "end" && start.slide == end.slide => {
                Ok(SlideSpan {
                    slide: start.slide.clone(),
                    start: start.page,
                    end: end.page,
                })
            }
            _ => anyhow::bail!("page markers are not in start/end pairs"),
        })
        .collect()
}

/// Prints the slides that did not fit on the page they were given.
///
/// Printed rather than logged: the default log filter drops warnings, and a
/// deck quietly gaining fifteen pages nobody would present is exactly what this
/// exists to stop.
#[allow(clippy::print_stdout)]
fn report_overflow(spans: &[SlideSpan]) {
    let overflowing = spans
        .iter()
        .filter(|span| span.pages() > 1)
        .collect::<Vec<_>>();
    if overflowing.is_empty() {
        return;
    }

    let pages = spans.iter().map(|span| span.end).max().unwrap_or_default();
    println!(
        "⚠️  {} of {} slides do not fit on one page — the deck is {pages} pages",
        overflowing.len(),
        spans.len(),
    );
    for span in overflowing {
        println!(
            "   {} — pages {}–{} ({} over)",
            span.slide,
            span.start,
            span.end,
            span.pages() - 1
        );
    }
    println!("   Shorten them, or give the deck a `_preamble.typ` with more room.");
}

/// Derives a default `<name>.pdf` path from the input folder, treating a
/// trailing `slides/` as the deck's `slides` directory.
fn default_pdf_path(input: &Path) -> PathBuf {
    let name = input
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| *name != "slides")
        .or_else(|| {
            input
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
        })
        .unwrap_or("presentation");
    PathBuf::from(format!("{name}.pdf"))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn marker(slide: &str, at: &str, page: usize) -> SlideMarker {
        SlideMarker {
            slide: slide.to_owned(),
            at: at.to_owned(),
            page,
        }
    }

    #[test]
    fn a_slide_that_fits_spans_one_page() {
        let spans = spans_from(&[marker("1-one.md", "start", 2), marker("1-one.md", "end", 2)])
            .expect("paired");

        assert_eq!(spans.len(), 1);
        assert_eq!(
            spans.first().expect("one span").pages(),
            1,
            "start and end on the same page"
        );
    }

    #[test]
    fn a_slide_that_overflows_spans_the_pages_it_took() {
        // The 23-slide deck that quietly became 38 pages: this is the number
        // that used to be nowhere in the output.
        let spans = spans_from(&[
            marker("1-one.md", "start", 2),
            marker("1-one.md", "end", 2),
            marker("2-two.md", "start", 3),
            marker("2-two.md", "end", 6),
        ])
        .expect("paired");

        assert_eq!(spans.get(1).expect("two spans").pages(), 4);
        assert_eq!(
            spans
                .iter()
                .filter(|span| span.pages() > 1)
                .map(|span| span.slide.as_str())
                .collect::<Vec<_>>(),
            ["2-two.md"],
            "only the slide that did not fit is reported"
        );
    }

    #[test]
    fn markers_that_do_not_pair_up_are_refused() {
        // Rather than pairing one slide's start with the next slide's end and
        // blaming the wrong file.
        let orphan = spans_from(&[marker("1-one.md", "start", 2)]);
        assert!(orphan.is_err(), "an unclosed slide is not a span");

        let crossed = spans_from(&[marker("1-one.md", "start", 2), marker("2-two.md", "end", 3)]);
        assert!(
            crossed.is_err(),
            "a start and end from two slides is not a span"
        );
    }
}
