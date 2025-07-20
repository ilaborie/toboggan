use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, bail};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use toboggan_core::{Content, Slide, SlideKind, Style, Talk};
use tracing::info;

mod settings;
pub use self::settings::*;

#[doc(hidden)]
#[allow(clippy::print_stderr)]
pub fn launch(settings: Settings) -> anyhow::Result<()> {
    info!(?settings, "launching server...");
    let Settings { output, input } = settings;

    let content =
        fs::read_to_string(&input).with_context(|| format!("reading {}", input.display()))?;
    let talk = parse_content(&content).context("parse content")?;

    let toml = toml::to_string_pretty(&talk).context("to TOML")?;

    if let Some(out) = &output {
        write_talk(out, &toml).context("write talk")?;
    } else {
        eprintln!("{toml}");
    }

    Ok(())
}

fn write_talk(out: &Path, toml: &str) -> anyhow::Result<()> {
    let writer = File::create(out).with_context(|| format!("creating {}", out.display()))?;
    let mut writer = BufWriter::new(writer);
    writer.write_all(toml.as_bytes()).context("writing data")?;

    Ok(())
}

fn parse_content(text: &str) -> anyhow::Result<Talk> {
    let mut state = TalkParseState::default();
    let options = Options::all();

    let parser = Parser::new_ext(text, options);
    for event in parser {
        state
            .consume(&event)
            .with_context(|| format!("processing {event:?}"))?;
    }

    let talk = state.finish().context("finish parsing")?;
    Ok(talk)
}

#[derive(Debug, Clone, Default)]
enum TalkParseState<'i> {
    #[default]
    Init,

    Title {
        current: Vec<Event<'i>>,
    },

    Slide {
        talk: Talk,
        current: Vec<Event<'i>>,
        is_first_slide: bool,
    },
}

impl<'i> TalkParseState<'i> {
    fn consume(&mut self, event: &Event<'i>) -> anyhow::Result<()> {
        match self {
            Self::Init => {
                if let Event::Start(Tag::Heading { level, .. }) = event
                    && level == &HeadingLevel::H1
                {
                    *self = Self::Title { current: vec![] };
                } else {
                    bail!("expected a heading level 1, got {event:?}");
                }
            }
            Self::Title { current } => {
                if let Event::End(TagEnd::Heading(level)) = event
                    && level == &HeadingLevel::H1
                {
                    let title = events_to_content(current);
                    let zoned = jiff::Zoned::now();
                    let date = zoned.date();
                    let talk = Talk {
                        title,
                        date,
                        slides: vec![],
                    };
                    *self = Self::Slide {
                        talk,
                        current: vec![],
                        is_first_slide: true,
                    };
                } else {
                    current.push(event.clone());
                }
            }
            Self::Slide {
                talk,
                current,
                is_first_slide,
            } => {
                if let Event::Rule = event {
                    if !current.is_empty() {
                        let mut slide = events_to_slide(current);
                        if *is_first_slide {
                            slide.kind = SlideKind::Cover;
                            *is_first_slide = false;
                        }
                        talk.slides.push(slide);
                        current.clear();
                    }
                } else {
                    current.push(event.clone());
                }
            }
        }

        Ok(())
    }

    fn finish(self) -> anyhow::Result<Talk> {
        match self {
            Self::Slide {
                mut talk,
                current,
                is_first_slide,
            } => {
                if !current.is_empty() {
                    let mut slide = events_to_slide(&current);
                    if is_first_slide {
                        slide.kind = SlideKind::Cover;
                    }
                    talk.slides.push(slide);
                }
                Ok(talk)
            }
            _ => bail!("invalid state: expected to be in Slide state at finish"),
        }
    }
}

fn events_to_content(events: &[Event]) -> Content {
    let mut text = String::new();

    for event in events {
        match event {
            Event::Text(text_content) => text.push_str(text_content),
            Event::Code(code_content) => {
                text.push('`');
                text.push_str(code_content);
                text.push('`');
            }
            Event::SoftBreak => text.push(' '),
            Event::HardBreak => text.push('\n'),
            _ => {}
        }
    }

    Content::Text {
        text: text.trim().to_string(),
    }
}

fn events_to_slide(events: &[Event]) -> Slide {
    let mut title_events = Vec::new();
    let mut body_events = Vec::new();
    let mut notes_events = Vec::new();
    let mut in_title = false;
    let mut in_notes = false;
    let mut slide_kind = SlideKind::Standard;

    for event in events {
        match event {
            Event::Start(Tag::Heading { level, classes, .. }) => match level {
                HeadingLevel::H2 => {
                    in_title = true;
                    slide_kind = SlideKind::Part;
                    continue;
                }
                HeadingLevel::H3 => {
                    in_title = true;
                    continue;
                }
                HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6 => {
                    if classes.iter().any(|class| class.as_ref() == "notes") {
                        in_notes = true;
                        continue;
                    }
                }
                HeadingLevel::H1 => {}
            },
            Event::End(TagEnd::Heading(level)) => {
                if matches!(level, HeadingLevel::H2 | HeadingLevel::H3) {
                    in_title = false;
                    continue;
                } else if matches!(
                    level,
                    HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6
                ) && in_notes
                {
                    in_notes = false;
                    continue;
                }
            }
            // Fallback to blockquote for notes (for compatibility)
            Event::Start(Tag::BlockQuote(_)) => {
                in_notes = true;
                continue;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_notes = false;
                continue;
            }
            _ => {}
        }

        if in_title {
            title_events.push(event.clone());
        } else if in_notes {
            notes_events.push(event.clone());
        } else {
            body_events.push(event.clone());
        }
    }

    let title = if title_events.is_empty() {
        Content::Empty
    } else {
        events_to_content(&title_events)
    };

    let body = if body_events.is_empty() {
        Content::Empty
    } else {
        events_to_markdown_content(&body_events)
    };

    let notes = if notes_events.is_empty() {
        Content::Empty
    } else {
        events_to_content(&notes_events)
    };

    Slide {
        kind: slide_kind,
        style: Style::default(),
        title,
        body,
        notes,
    }
}

fn events_to_markdown_content(events: &[Event]) -> Content {
    use pulldown_cmark::html;

    let mut html = String::new();
    html::push_html(&mut html, events.iter().cloned());

    let alt = events_to_markdown_text(events);

    if html.trim().is_empty() {
        Content::Empty
    } else {
        Content::Html {
            raw: html.trim().to_string(),
            alt: if alt.is_empty() { None } else { Some(alt) },
        }
    }
}

fn events_to_markdown_text(events: &[Event]) -> String {
    let mut markdown = String::new();
    let mut list_depth: usize = 0;
    let mut in_code_block = false;

    for event in events {
        match event {
            Event::Start(Tag::Paragraph) => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item) => {
                markdown.push('\n');
            }
            Event::Start(Tag::Heading { level, .. }) => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                match level {
                    pulldown_cmark::HeadingLevel::H1 => markdown.push('#'),
                    pulldown_cmark::HeadingLevel::H2 => markdown.push_str("##"),
                    pulldown_cmark::HeadingLevel::H3 => markdown.push_str("###"),
                    pulldown_cmark::HeadingLevel::H4 => markdown.push_str("####"),
                    pulldown_cmark::HeadingLevel::H5 => markdown.push_str("#####"),
                    pulldown_cmark::HeadingLevel::H6 => markdown.push_str("######"),
                }
                markdown.push(' ');
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                for _ in 0..(list_depth - 1) {
                    markdown.push_str("  ");
                }
                markdown.push_str("- ");
            }
            Event::Start(Tag::CodeBlock(_)) => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push_str("```\n");
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                if !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push_str("```\n");
                in_code_block = false;
            }
            Event::Text(text) => {
                markdown.push_str(text);
            }
            Event::Code(code) => {
                if in_code_block {
                    markdown.push_str(code);
                } else {
                    markdown.push('`');
                    markdown.push_str(code);
                    markdown.push('`');
                }
            }
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => markdown.push_str("**"),
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => markdown.push('*'),
            Event::SoftBreak => markdown.push(' '),
            Event::HardBreak => markdown.push('\n'),
            Event::Rule => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push_str("---\n");
            }
            _ => {}
        }
    }

    markdown.trim().to_string()
}
