use std::path::{Path, PathBuf};

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toboggan_core::{SlideKind, Talk};
use toboggan_lint::LintConfig;
use toboggan_stats::SlideStats;

use crate::workspace::{ChangeSet, OutlineNode, Workspace};

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
        description = "Outline the presentation: the cover, parts, and slides as they exist on \
                          disk, each with the relative `path` that the editing tools address, \
                          plus titles, hidden-in targets, and skip flags."
    )]
    fn talk_outline(&self) -> Result<Json<Outline>, ErrorData> {
        let talk = self.load_talk()?;
        let nodes = self
            .workspace()?
            .outline()
            .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
        Ok(Json(Outline {
            title: talk.title.clone(),
            date: talk.date.to_string(),
            nodes,
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

    #[tool(
        description = "Lint the presentation and return the diagnostics report as JSON. Set \
                          `spell` to also run the spell checker."
    )]
    fn lint(&self, Parameters(params): Parameters<Lint>) -> Result<Json<LintResult>, ErrorData> {
        let talk = self.load_talk()?;
        let mut config = LintConfig::default();
        // `spelling/typo` shells out to the `typos` binary over every slide, so
        // it is opt-in here: an agent linting after each edit should not pay for
        // a whole-deck spell check it did not ask for. (The rule is compiled in
        // whenever anything in the binary enables `toboggan-lint/spell`, which
        // the CLI does — so it has to be disabled explicitly, not by omission.)
        if !params.spell {
            config.disable(toboggan_lint::ids::SPELLING_TYPO);
        }
        let report = toboggan_lint::lint(&talk, &config);
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
        let change = workspace.add_part(&params.title).map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("added part \"{}\"", params.title),
            change,
        ))
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
        let change = workspace
            .add_slide(
                params.part.as_deref(),
                &params.title,
                params.body.as_deref(),
            )
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("added slide \"{}\"", params.title),
            change,
        ))
    }

    #[tool(
        description = "Scaffold a complete new deck folder (its own slides/, public/, mise.toml) \
                          at the subdirectory `dir`, relative to the server's root."
    )]
    fn new_presentation(
        &self,
        Parameters(params): Parameters<NewPresentation>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let date = match params.date {
            Some(date) => date
                .parse::<toboggan_core::Date>()
                .map_err(|_| ErrorData::invalid_params("date must be YYYY-MM-DD", None))?,
            None => toboggan_core::Date::today(),
        };
        let change = self
            .workspace()?
            .new_presentation(&params.dir, &params.title, date)
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("scaffolded presentation \"{}\"", params.title),
            change,
        ))
    }

    #[tool(description = "Set a slide's front-matter title. Pass `dry_run` to preview.")]
    fn set_slide_title(
        &self,
        Parameters(params): Parameters<SetTitle>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .set_slide_title(&params.path, &params.title, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("set title of {}", params.path),
            change,
        ))
    }

    #[tool(
        description = "Set a section's title by editing its _part.md. Pass `dry_run` to preview."
    )]
    fn set_part_title(
        &self,
        Parameters(params): Parameters<SetPartTitle>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .set_part_title(&params.folder, &params.title, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("set title of {}", params.folder),
            change,
        ))
    }

    #[tool(
        description = "Replace a slide's markdown body, preserving its front matter. Pass `dry_run` to preview."
    )]
    fn set_slide_body(
        &self,
        Parameters(params): Parameters<SetBody>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .set_slide_body(&params.path, &params.body, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("set body of {}", params.path),
            change,
        ))
    }

    #[tool(
        description = "Set the render targets a slide is hidden in (each must be \"web\" or \
                          \"pdf\"; an empty list makes it visible everywhere). Pass `dry_run` \
                          to preview."
    )]
    fn set_hidden_in(
        &self,
        Parameters(params): Parameters<SetHiddenIn>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .set_hidden_in(&params.path, &params.targets, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("set hidden_in of {}", params.path),
            change,
        ))
    }

    #[tool(
        description = "Toggle a slide's `skip` flag (a skipped slide is omitted from the built \
                          talk). Pass `dry_run` to preview."
    )]
    fn skip_slide(
        &self,
        Parameters(params): Parameters<SkipSlide>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .skip_slide(&params.path, params.skip, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("set skip of {}", params.path),
            change,
        ))
    }

    #[tool(description = "Delete a slide file. Pass `dry_run` to preview.")]
    fn remove_slide(
        &self,
        Parameters(params): Parameters<RemovePath>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .remove_slide(&params.path, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("removed {}", params.path),
            change,
        ))
    }

    #[tool(
        description = "Delete a section folder and all of its slides. Pass `dry_run` to preview."
    )]
    fn remove_part(
        &self,
        Parameters(params): Parameters<RemoveFolder>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .remove_part(&params.folder, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(
            format!("removed {}", params.folder),
            change,
        ))
    }

    #[tool(
        description = "Reorder entries by renumbering them. With `part`, reorders that section's \
                          slides; without it, reorders top-level parts and slides. `order` must \
                          list the directory's current numbered names in the desired sequence. \
                          Pass `dry_run` to preview."
    )]
    fn reorder(
        &self,
        Parameters(params): Parameters<Reorder>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .reorder(params.part.as_deref(), &params.order, params.dry_run.into())
            .map_err(invalid_params)?;
        Ok(ChangeResult::json("reordered".to_owned(), change))
    }

    #[tool(
        description = "Move a slide to another section (or the top level with no `to_part`) at an \
                          optional 1-based `position`, renumbering both directories. Pass \
                          `dry_run` to preview."
    )]
    fn move_slide(
        &self,
        Parameters(params): Parameters<MoveSlide>,
    ) -> Result<Json<ChangeResult>, ErrorData> {
        let change = self
            .workspace()?
            .move_slide(
                &params.from,
                params.to_part.as_deref(),
                params.position,
                params.dry_run.into(),
            )
            .map_err(invalid_params)?;
        Ok(ChangeResult::json(format!("moved {}", params.from), change))
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

// `#[tool_handler]` generates `call_tool` as an `async fn` that never awaits.
// The lint is right, but the body is rmcp's to write, not ours. `unknown_lints`
// rides along because the lint only exists from clippy 1.98 and nothing here
// pins a toolchain.
#[allow(unknown_lints, clippy::unused_async_trait_impl)]
#[tool_handler]
impl ServerHandler for TobogganServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Authoring tools for a Toboggan presentation folder (the server's root is the \
             slides folder). Start with `talk_outline`: it lists the cover, parts, and slides \
             with the relative `path` every editing tool addresses. Inspect quality with \
             `stats`/`lint` and read `advice` for conventions. Edit with `add_part`/`add_slide`, \
             `set_slide_title`/`set_part_title`/`set_slide_body`, `set_hidden_in`/`skip_slide`, \
             `remove_slide`/`remove_part`, and reorganize with `reorder`/`move_slide`. \
             Mutating tools accept `dry_run` to preview the change set first — use it before \
             `reorder`/`move_slide`. Prefer these tools over editing files directly.",
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
        lang: None,
        base_url: None,
        theme: "base16-ocean.light".to_owned(),
        mermaid_config: None,
        typst_preamble: None,
        list_themes: false,
        format: None,
        no_counter: false,
        no_stats: true,
        wpm: 150,
        exclude_notes_from_duration: false,
        input: Some(root.to_path_buf()),
    }
}

/// Maps a workspace mutation error to an MCP invalid-params error.
fn invalid_params(err: impl std::fmt::Display) -> ErrorData {
    ErrorData::invalid_params(err.to_string(), None)
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Outline {
    pub(crate) title: String,
    pub(crate) date: String,
    /// The cover, parts, and slides as they exist on disk.
    pub(crate) nodes: Vec<OutlineNode>,
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
    pub(crate) change: ChangeSet,
}

impl ChangeResult {
    fn json(message: String, change: ChangeSet) -> Json<Self> {
        Json(Self { message, change })
    }
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

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct Lint {
    /// Also run the spell checker (`spelling/typo`). Off by default: it shells
    /// out to the `typos` binary over every slide.
    #[serde(default)]
    pub(crate) spell: bool,
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

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct NewPresentation {
    /// Directory (relative to the server root) to scaffold the presentation in.
    pub(crate) dir: String,
    /// Presentation title.
    pub(crate) title: String,
    /// Optional date (YYYY-MM-DD; defaults to today).
    pub(crate) date: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetTitle {
    /// Relative path of the slide file (from `talk_outline`).
    pub(crate) path: String,
    /// New title.
    pub(crate) title: String,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetPartTitle {
    /// Folder name of the section.
    pub(crate) folder: String,
    /// New title.
    pub(crate) title: String,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetBody {
    /// Relative path of the slide file.
    pub(crate) path: String,
    /// New markdown body.
    pub(crate) body: String,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetHiddenIn {
    /// Relative path of the slide file.
    pub(crate) path: String,
    /// Render targets to hide the slide in (`"web"` and/or `"pdf"`; empty = visible).
    pub(crate) targets: Vec<String>,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SkipSlide {
    /// Relative path of the slide file.
    pub(crate) path: String,
    /// Whether to skip (exclude) the slide.
    pub(crate) skip: bool,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RemovePath {
    /// Relative path of the slide file.
    pub(crate) path: String,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RemoveFolder {
    /// Folder name of the section.
    pub(crate) folder: String,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct Reorder {
    /// Section folder to reorder within; omit to reorder top-level parts/slides.
    pub(crate) part: Option<String>,
    /// The directory's current numbered names, in the desired order.
    pub(crate) order: Vec<String>,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct MoveSlide {
    /// Relative path of the slide to move.
    pub(crate) from: String,
    /// Target section folder; omit to move to the top level.
    pub(crate) to_part: Option<String>,
    /// 1-based position in the target; omit to append.
    pub(crate) position: Option<usize>,
    /// Preview without writing.
    #[serde(default)]
    pub(crate) dry_run: bool,
}
