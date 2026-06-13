use std::path::{Path, PathBuf};

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toboggan_core::{SlideKind, Talk};
use toboggan_lint::LintConfig;
use toboggan_stats::SlideStats;

use crate::workspace::Workspace;

/// MCP server exposing Toboggan authoring tools over a presentation folder.
///
/// `#[tool_handler]` routes calls via the generated `Self::tool_router()`, so no
/// router field is needed.
#[derive(Clone)]
pub struct TobogganServer {
    root: PathBuf,
}

impl TobogganServer {
    /// Creates a server operating on the presentation folder `root`.
    #[must_use]
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace(&self) -> Result<Workspace, ErrorData> {
        Workspace::new(&self.root).map_err(|err| ErrorData::internal_error(err.to_string(), None))
    }

    fn load_talk(&self) -> Result<Talk, ErrorData> {
        load_talk(&self.root).map_err(|err| ErrorData::internal_error(err.to_string(), None))
    }
}

#[tool_router]
impl TobogganServer {
    #[tool(
        description = "List the parts and slides of the presentation with their indices, \
                          kinds, titles, and web-hidden flags."
    )]
    fn talk_outline(&self) -> Result<Json<Outline>, ErrorData> {
        let talk = self.load_talk()?;
        let slides = talk
            .slides
            .iter()
            .enumerate()
            .map(|(index, slide)| OutlineSlide {
                index,
                display_number: index + 1,
                kind: format!("{:?}", slide.kind),
                title: slide.title.to_string(),
                hidden_in_web: slide.hidden_in.contains(&toboggan_core::RenderTarget::Web),
            })
            .collect();
        Ok(Json(Outline {
            title: talk.title.clone(),
            date: talk.date.to_string(),
            slides,
        }))
    }

    #[tool(description = "Compute presentation statistics: slide counts and total word count.")]
    fn stats(&self) -> Result<Json<StatsSummary>, ErrorData> {
        let talk = self.load_talk()?;
        let words = talk
            .slides
            .iter()
            .map(|slide| SlideStats::from_slide(slide).words)
            .sum();
        let parts = talk
            .slides
            .iter()
            .filter(|slide| slide.kind == SlideKind::Part)
            .count();
        Ok(Json(StatsSummary {
            total_slides: talk.slides.len(),
            content_slides: talk
                .slides
                .iter()
                .filter(|slide| slide.kind != SlideKind::Part)
                .count(),
            parts,
            words,
        }))
    }

    #[tool(description = "Lint the presentation and return the diagnostics report as JSON.")]
    fn lint(&self) -> Result<Json<LintResult>, ErrorData> {
        let talk = self.load_talk()?;
        let report = toboggan_lint::lint(&talk, &LintConfig::default());
        let report = serde_json::to_value(&report)
            .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
        Ok(Json(LintResult { report }))
    }

    #[tool(
        description = "Add a new section (part) to the presentation. Creates a numbered \
                          subfolder with a _part.md."
    )]
    fn add_part(
        &self,
        Parameters(params): Parameters<AddPart>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let workspace = self.workspace()?;
        let created = workspace
            .add_part(&params.title)
            .map_err(|err| ErrorData::invalid_params(err.to_string(), None))?;
        Ok(Json(ChangeResult {
            message: format!("added part \"{}\"", params.title),
            created,
        }))
    }

    #[tool(
        description = "Add a new slide. If `part` is given (its folder name), the slide is \
                          created in that section; otherwise at the top level."
    )]
    fn add_slide(
        &self,
        Parameters(params): Parameters<AddSlide>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let workspace = self.workspace()?;
        let created = workspace
            .add_slide(
                params.part.as_deref(),
                &params.title,
                params.body.as_deref(),
            )
            .map_err(|err| ErrorData::invalid_params(err.to_string(), None))?;
        Ok(Json(ChangeResult {
            message: format!("added slide \"{}\"", params.title),
            created,
        }))
    }

    #[tool(
        description = "Get authoring guidance for Toboggan presentations (folder layout, \
                          pause/notes/terminal directives, styling)."
    )]
    #[allow(clippy::unused_self)]
    fn advice(&self) -> Json<Advice> {
        Json(Advice {
            advice: crate::ADVICE.to_owned(),
        })
    }
}

#[tool_handler]
impl ServerHandler for TobogganServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Authoring tools for a Toboggan presentation. Use `talk_outline` to inspect \
             structure, `stats`/`lint` to check quality, and `add_part`/`add_slide` to edit \
             the folder. Prefer these tools over editing files directly.",
        )
    }
}

/// Loads the talk from the presentation folder `root`.
fn load_talk(root: &Path) -> anyhow::Result<Talk> {
    let settings = default_settings(root);
    let parse_result = toboggan_cli::parse_presentation(root, &settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(parse_result.to_talk())
}

fn default_settings(root: &Path) -> toboggan_cli::Settings {
    toboggan_cli::Settings {
        output: None,
        title: None,
        date: None,
        theme: "base16-ocean.light".to_owned(),
        list_themes: false,
        format: None,
        no_counter: false,
        no_stats: true,
        wpm: 150,
        exclude_notes_from_duration: false,
        input: Some(root.to_path_buf()),
    }
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Outline {
    pub(crate) title: String,
    pub(crate) date: String,
    pub(crate) slides: Vec<OutlineSlide>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct OutlineSlide {
    pub(crate) index: usize,
    pub(crate) display_number: usize,
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) hidden_in_web: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct StatsSummary {
    pub(crate) total_slides: usize,
    pub(crate) content_slides: usize,
    pub(crate) parts: usize,
    pub(crate) words: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ChangeResult {
    pub(crate) message: String,
    pub(crate) created: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Advice {
    pub(crate) advice: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct LintResult {
    /// The full lint report (diagnostics + severity counts).
    pub(crate) report: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AddPart {
    /// Title of the new section.
    pub(crate) title: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AddSlide {
    /// Title of the new slide.
    pub(crate) title: String,
    /// Optional folder name of the section to add the slide to.
    pub(crate) part: Option<String>,
    /// Optional markdown body for the slide.
    pub(crate) body: Option<String>,
}
