use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use anyhow::{Context, bail};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use toboggan_core::{Slide, Talk};
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
    // options.set(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS, false);

    let parser = Parser::new_ext(text, options);
    for event in parser {
        state
            .consume(&event)
            .with_context(|| format!("processing {event:?}"))?;
    }

    dbg!(&state);

    let talk = state.finish().context("finish parsing")?;
    // };
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
                    // TODO create the title, and create the talk with the current date
                    // TODO switch to the Self::Slide state
                } else {
                    // TODO we fill the current list of event
                }
            }
            Self::Slide { talk, current } => {
                // TODO if Event is Rule, build the slide with current add the slide to the talk, and clear the current
                // TODO otherwise fill the current list of event
                todo!()
            }
        }

        Ok(())
    }

    fn finish(self) -> anyhow::Result<Talk> {
        // TODO it Self::Slide, finish the build of the talk by adding the last slide
        // else fail because we are a an invalid state
        todo!()
    }
}
