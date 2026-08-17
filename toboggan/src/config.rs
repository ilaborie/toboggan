//! The `toboggan.toml` configuration layer.
//!
//! Everything the CLI can be told with a flag, it can also be told with a file,
//! so a deck carries its own settings instead of every author retyping
//! `--theme … --port … --wpm …` on each invocation.
//!
//! # Discovery
//!
//! Starting at the deck directory, each directory up to the filesystem root is
//! checked for [`CONFIG_NAMES`] (the dotted name wins within a directory), and
//! finally the user-global file under `$XDG_CONFIG_HOME` (or `~/.config`). Every
//! file found contributes; the *nearest* one wins per field, so a repo-wide file
//! can set a house theme while one deck overrides the port.
//!
//! # Precedence
//!
//! `CLI flag > environment variable > nearest config > … > user-global > default`.
//!
//! The first two come free from clap, but only because the flags this file can
//! override are declared as `Option<T>` with no `default_value`: a clap default
//! is indistinguishable from a value the user typed, and would silently outrank
//! every config file. Defaults are therefore applied *after* merging, by the
//! `resolve` methods in [`crate::cli`].
//!
//! # Failure
//!
//! A malformed or unreadable config is a hard error naming the file. Every
//! struct is `deny_unknown_fields`, so a typo'd key is reported rather than
//! ignored — a silently-dropped setting is worse than a failed run.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde::Deserialize;
use toboggan_core::Date;
use toboggan_lint::Severity;

use crate::cli::DenyLevel;

/// Accepted file names within one directory, most-specific first.
const CONFIG_NAMES: [&str; 2] = [".toboggan.toml", "toboggan.toml"];

/// The user-global config, relative to `$XDG_CONFIG_HOME` (or `~/.config`).
const USER_CONFIG_RELATIVE: &str = "toboggan/config.toml";

/// What a bare `toboggan` does when no subcommand is given.
///
/// Deliberately not every subcommand: a config file that could make `toboggan`
/// scaffold files, install an MCP server, or open a client would turn an
/// innocuous command into a surprising one. Only read-only and serve-like
/// commands are selectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DefaultCommand {
    /// Build the deck in memory and serve it, watching for changes.
    #[default]
    Serve,
    /// Lint the deck.
    Lint,
    /// Print deck statistics.
    Stats,
    /// Build the deck to a file.
    Build,
    /// Render the deck to a PDF.
    Pdf,
    /// Generate per-slide thumbnails and an overview page.
    Thumbnails,
}

/// One `toboggan.toml`, or several merged together.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Config {
    /// Deck directory, resolved relative to the file that declares it.
    pub(crate) path: Option<PathBuf>,

    /// Which command a bare `toboggan` runs.
    pub(crate) default_command: Option<DefaultCommand>,

    #[serde(default)]
    pub(crate) build: BuildConfig,

    #[serde(default)]
    pub(crate) serve: ServeConfig,

    #[serde(default)]
    pub(crate) lint: LintConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct BuildConfig {
    pub(crate) title: Option<String>,
    pub(crate) date: Option<Date>,
    pub(crate) theme: Option<String>,
    pub(crate) no_counter: Option<bool>,
    pub(crate) wpm: Option<u16>,
    pub(crate) exclude_notes_from_duration: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ServeConfig {
    pub(crate) host: Option<IpAddr>,
    pub(crate) port: Option<u16>,
    pub(crate) max_clients: Option<usize>,
    pub(crate) allowed_origins: Option<Vec<String>>,
    pub(crate) public_dir: Option<PathBuf>,
    pub(crate) thumbnails_dir: Option<PathBuf>,
    pub(crate) shell: Option<String>,
    pub(crate) open: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct LintConfig {
    pub(crate) deny: Option<DenyLevel>,
    pub(crate) no_spell: Option<bool>,
    pub(crate) max_steps_per_slide: Option<usize>,
    pub(crate) max_words_per_slide: Option<usize>,
    pub(crate) max_images_per_slide: Option<usize>,
    pub(crate) max_code_lines: Option<usize>,
    /// Rule ids to switch off. Unknown ids are reported by the linter itself.
    pub(crate) disabled: Option<Vec<String>>,
    /// Per-rule severity overrides, keyed by rule id.
    pub(crate) severity: Option<BTreeMap<String, Severity>>,
}

/// Fills every `None` field from `weaker`, leaving set fields alone.
///
/// One method per struct rather than a derive: the set is small, and an explicit
/// list makes a forgotten field show up in review instead of at runtime.
macro_rules! fill {
    ($self:ident, $weaker:ident, $($field:ident),+ $(,)?) => {
        $(if $self.$field.is_none() {
            $self.$field = $weaker.$field;
        })+
    };
}

impl Config {
    /// Layers `weaker` underneath `self` — `self` is the nearer file and wins.
    fn fill_from(&mut self, weaker: Self) {
        fill!(self, weaker, path, default_command);
        self.build.fill_from(weaker.build);
        self.serve.fill_from(weaker.serve);
        self.lint.fill_from(weaker.lint);
    }

    /// The deck directory this config points at, if it sets one.
    ///
    /// Relative paths resolve against the directory holding the config file, not
    /// the process cwd, so a repo-root config saying `path = "slides"` means the
    /// same thing regardless of where `toboggan` is run from.
    pub(crate) fn deck_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl BuildConfig {
    fn fill_from(&mut self, weaker: Self) {
        fill!(
            self,
            weaker,
            title,
            date,
            theme,
            no_counter,
            wpm,
            exclude_notes_from_duration,
        );
    }
}

impl ServeConfig {
    fn fill_from(&mut self, weaker: Self) {
        fill!(
            self,
            weaker,
            host,
            port,
            max_clients,
            allowed_origins,
            public_dir,
            thumbnails_dir,
            shell,
            open,
        );
    }
}

impl LintConfig {
    fn fill_from(&mut self, weaker: Self) {
        fill!(
            self,
            weaker,
            deny,
            no_spell,
            max_steps_per_slide,
            max_words_per_slide,
            max_images_per_slide,
            max_code_lines,
            disabled,
            severity,
        );
    }
}

/// Loads and merges every config that applies to a deck at `start`.
///
/// # Errors
/// Returns an error if a config file exists but cannot be read or parsed.
pub(crate) fn load(start: &Path) -> anyhow::Result<Config> {
    load_layers(start, user_config_path().as_deref())
}

/// The testable core of [`load`], with the user-global path injected.
///
/// Taking the global path as a parameter rather than reading the environment
/// keeps the tests from mutating process-global state (`set_var` is `unsafe` in
/// edition 2024) and lets them cover the global layer at all.
fn load_layers(start: &Path, user_config: Option<&Path>) -> anyhow::Result<Config> {
    // Absolute first, or the walk goes nowhere: `Path::new(".").ancestors()`
    // yields just "." and "", so a relative start would only ever see the
    // current directory and silently ignore every parent config.
    let start = absolute(start);

    // Nearest first: `fill_from` only ever fills gaps, so folding in this order
    // means the first file to set a field keeps it.
    let mut merged = Config::default();
    for dir in start.ancestors() {
        if let Some(found) = read_dir_config(dir)? {
            merged.fill_from(found);
        }
    }
    if let Some(path) = user_config
        && let Some(found) = read_config(path)?
    {
        merged.fill_from(found);
    }
    Ok(merged)
}

/// Makes `path` absolute so that [`Path::ancestors`] can actually walk it.
///
/// `canonicalize` when the directory exists (it also resolves symlinks, so two
/// spellings of the same deck find the same configs); otherwise fall back to
/// joining the cwd, since the deck may not exist yet and that should surface as
/// "not a directory" rather than as a config-loading failure.
fn absolute(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
    })
}

/// Reads the config in `dir`, if any. The dotted name wins over the plain one.
fn read_dir_config(dir: &Path) -> anyhow::Result<Option<Config>> {
    for name in CONFIG_NAMES {
        if let Some(config) = read_config(&dir.join(name))? {
            return Ok(Some(config));
        }
    }
    Ok(None)
}

/// Rebases a relative `path` onto the directory holding the config that set it.
///
/// Without this, a repo-root config saying `path = "slides"` would mean
/// "./slides relative to wherever you happen to have run `toboggan`", which is
/// the same cwd-relative trap the deck resolution already had to fix once.
fn anchor_path(config: &mut Config, file: &Path) {
    let Some(declared) = config.path.as_ref() else {
        return;
    };
    if declared.is_relative()
        && let Some(dir) = file.parent()
    {
        config.path = Some(dir.join(declared));
    }
}

/// Reads one config file, returning `None` when it simply does not exist.
///
/// Any *other* I/O error is propagated: an unreadable config is a real problem,
/// and treating "permission denied" as "no config" would silently ignore
/// settings the author expected to apply.
fn read_config(path: &Path) -> anyhow::Result<Option<Config>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(anyhow::Error::new(err).context(format!("reading {}", path.display())));
        }
    };
    let mut config =
        toml::from_str::<Config>(&text).with_context(|| format!("parsing {}", path.display()))?;
    anchor_path(&mut config, path);
    tracing::debug!(config = %path.display(), "loaded configuration");
    Ok(Some(config))
}

/// `$XDG_CONFIG_HOME/toboggan/config.toml`, falling back to `~/.config`.
///
/// XDG semantics rather than the platform-native location: `toboggan` is a
/// developer tool driven from a shell, and on macOS the native answer
/// (`~/Library/Application Support`) is not where anyone looks for a CLI's
/// dotfile.
fn user_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join(USER_CONFIG_RELATIVE))
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
