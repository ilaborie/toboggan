#[cfg(test)]
#[allow(clippy::module_inception, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use crate::cli::DenyLevel;
    use crate::config::{DefaultCommand, load_layers};

    /// Writes `contents` to `dir/name`, creating `dir` first.
    fn write(dir: &Path, name: &str, contents: &str) {
        std::fs::create_dir_all(dir).expect("create dir");
        std::fs::write(dir.join(name), contents).expect("write config");
    }

    #[test]
    fn no_config_anywhere_yields_defaults() {
        let root = TempDir::new().expect("temp dir");
        let config = load_layers(root.path(), None).expect("load");

        assert!(config.default_command.is_none());
        assert!(config.serve.port.is_none());
        assert!(config.build.theme.is_none());
    }

    #[test]
    fn nearest_config_wins_and_farther_ones_fill_the_gaps() {
        let root = TempDir::new().expect("temp dir");
        let deck = root.path().join("decks").join("my-talk");
        write(
            root.path(),
            "toboggan.toml",
            "[build]\ntheme = \"house-style\"\nwpm = 130\n[serve]\nport = 1111\n\
             [overview]\nthumbnail-renderer = \"browser\"\nbrowser = \"/opt/chrome\"\n",
        );
        write(
            &deck,
            "toboggan.toml",
            "[serve]\nport = 9999\n[overview]\nthumbnail-renderer = \"typst\"\n",
        );

        let config = load_layers(&deck, None).expect("load");

        // The deck sets the port, so it wins over the repo-root value.
        assert_eq!(config.serve.port, Some(9999));
        // It says nothing about the theme or wpm, which fall through.
        assert_eq!(config.build.theme.as_deref(), Some("house-style"));
        assert_eq!(config.build.wpm, Some(130));
        // `[overview]` layers like every other table. It was left out of
        // `fill_from` entirely, so *every* config file's `[overview]` was
        // dropped on the floor before it ever reached a command.
        assert_eq!(
            config.overview.thumbnail_renderer,
            Some(toboggan_server::ThumbnailRenderer::Typst)
        );
        assert_eq!(
            config.overview.browser.as_deref(),
            Some(Path::new("/opt/chrome"))
        );
    }

    #[test]
    fn user_global_is_the_weakest_layer() {
        let root = TempDir::new().expect("temp dir");
        let home = TempDir::new().expect("temp home");
        let global = home.path().join("config.toml");
        std::fs::write(&global, "[build]\ntheme = \"global\"\nwpm = 100\n").expect("write global");
        write(root.path(), "toboggan.toml", "[build]\ntheme = \"repo\"\n");

        let config = load_layers(root.path(), Some(&global)).expect("load");

        assert_eq!(config.build.theme.as_deref(), Some("repo"));
        // Only the field the repo config leaves unset comes from the global one.
        assert_eq!(config.build.wpm, Some(100));
    }

    #[test]
    fn dotted_name_wins_within_a_directory() {
        let root = TempDir::new().expect("temp dir");
        write(root.path(), ".toboggan.toml", "[serve]\nport = 1234\n");
        write(root.path(), "toboggan.toml", "[serve]\nport = 4321\n");

        let config = load_layers(root.path(), None).expect("load");

        assert_eq!(config.serve.port, Some(1234));
    }

    #[test]
    fn unknown_key_is_an_error_rather_than_silently_ignored() {
        let root = TempDir::new().expect("temp dir");
        write(root.path(), "toboggan.toml", "[serve]\nprot = 8080\n");

        let err = load_layers(root.path(), None).expect_err("typo must fail");
        let rendered = format!("{err:?}");

        assert!(
            rendered.contains("prot"),
            "the error should name the offending key, got: {rendered}"
        );
    }

    #[test]
    fn malformed_toml_names_the_file() {
        let root = TempDir::new().expect("temp dir");
        write(root.path(), "toboggan.toml", "[serve\nport = 8080\n");

        let err = load_layers(root.path(), None).expect_err("malformed must fail");
        let rendered = format!("{err:?}");

        assert!(
            rendered.contains("toboggan.toml"),
            "the error should name the file, got: {rendered}"
        );
    }

    #[test]
    fn default_command_and_deny_level_parse_from_kebab_case() {
        let root = TempDir::new().expect("temp dir");
        write(
            root.path(),
            "toboggan.toml",
            "default-command = \"lint\"\n[lint]\ndeny = \"warning\"\nmax-words-per-slide = 42\n",
        );

        let config = load_layers(root.path(), None).expect("load");

        assert_eq!(config.default_command, Some(DefaultCommand::Lint));
        assert_eq!(config.lint.deny, Some(DenyLevel::Warning));
        assert_eq!(config.lint.max_words_per_slide, Some(42));
    }

    #[test]
    fn default_command_rejects_a_command_outside_the_safe_list() {
        let root = TempDir::new().expect("temp dir");
        write(root.path(), "toboggan.toml", "default-command = \"new\"\n");

        let err = load_layers(root.path(), None).expect_err("`new` must not be selectable");
        let rendered = format!("{err:?}");

        assert!(
            rendered.contains("default-command") || rendered.contains("unknown variant"),
            "the error should point at default-command, got: {rendered}"
        );
    }

    #[test]
    fn deck_path_and_lint_tables_round_trip() {
        let root = TempDir::new().expect("temp dir");
        write(
            root.path(),
            "toboggan.toml",
            "path = \"slides\"\n[lint]\ndisabled = [\"pause/in-part\"]\n",
        );

        let config = load_layers(root.path(), None).expect("load");

        assert_eq!(
            config.lint.disabled.as_deref(),
            Some(["pause/in-part".to_owned()].as_slice())
        );
        // A relative `path` is anchored to the config's own directory, not the
        // process cwd, so it means the same thing from anywhere.
        let expected = root
            .path()
            .canonicalize()
            .expect("canonicalize")
            .join("slides");
        assert_eq!(config.deck_path(), Some(expected.as_path()));
    }

    #[test]
    fn a_relative_deck_path_is_anchored_to_the_config_that_declared_it() {
        let root = TempDir::new().expect("temp dir");
        let deck = root.path().join("decks").join("my-talk");
        // Only the repo-root config names a path; the deck itself says nothing.
        write(root.path(), "toboggan.toml", "path = \"shared-slides\"\n");
        std::fs::create_dir_all(&deck).expect("create deck dir");

        let config = load_layers(&deck, None).expect("load");

        let expected = root
            .path()
            .canonicalize()
            .expect("canonicalize")
            .join("shared-slides");
        assert_eq!(
            config.deck_path(),
            Some(expected.as_path()),
            "the path must anchor to the repo root, not the deck it was found from"
        );
    }

    #[test]
    fn a_relative_typst_preamble_is_anchored_to_the_config_that_declared_it() {
        let root = TempDir::new().expect("temp dir");
        let deck = root.path().join("decks").join("my-talk");
        write(
            root.path(),
            "toboggan.toml",
            "[build]\ntypst-preamble = \"house-style.typ\"\n",
        );
        std::fs::create_dir_all(&deck).expect("create deck dir");

        let config = load_layers(&deck, None).expect("load");

        let expected = root
            .path()
            .canonicalize()
            .expect("canonicalize")
            .join("house-style.typ");
        assert_eq!(
            config.build.typst_preamble.as_deref(),
            Some(expected.as_path()),
            "a house preamble must resolve against the config that named it, \
             not against wherever toboggan was run from"
        );
    }

    /// The `toboggan.toml` that `toboggan new` writes, with every key commented
    /// out. It is the primary documentation of the config surface.
    const TEMPLATE: &str = include_str!("../../toboggan-cli/templates/toboggan.toml");

    /// Uncomments the key/table lines of the template, leaving prose comments.
    ///
    /// Matches `# key = …`, `# "quoted.key" = …` and `# [table]`; ordinary
    /// sentences start with a word and no `=`, so they stay comments.
    fn uncomment(template: &str) -> String {
        template
            .lines()
            .map(|line| {
                let Some(rest) = line.strip_prefix("# ") else {
                    return line.to_owned();
                };
                let is_table = rest.starts_with('[');
                let is_assignment = rest.split_once(" = ").is_some_and(|(key, _)| {
                    !key.contains(' ')
                        && (key.starts_with('"') || key.starts_with(char::is_lowercase))
                });
                if is_table || is_assignment {
                    rest.to_owned()
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The scaffolded config documents every setting by listing it commented
    /// out — which means nothing type-checks those key names. A typo there would
    /// stay invisible until an author uncommented the line and hit
    /// `deny_unknown_fields`. Uncommenting the whole file here keeps the
    /// template honest against the structs.
    #[test]
    fn every_key_in_the_scaffolded_template_is_a_real_setting() {
        let filled = TEMPLATE
            .replace("{{title}}", "My Talk")
            .replace("{{date}}", "2026-01-01");
        let enabled = uncomment(&filled);

        let config = toml::from_str::<crate::config::Config>(&enabled)
            .unwrap_or_else(|err| panic!("template is not valid config: {err}\n---\n{enabled}"));

        // Spot-check across all three tables, so an accidentally-empty parse
        // (everything still commented) cannot pass this test.
        assert_eq!(config.default_command, Some(DefaultCommand::Serve));
        assert_eq!(config.build.wpm, Some(150));
        assert_eq!(config.serve.port, Some(8080));
        assert_eq!(config.lint.deny, Some(DenyLevel::Error));
        assert_eq!(config.lint.max_words_per_slide, Some(120));
    }

    /// A relative start path used to make the ancestor walk a no-op:
    /// `Path::new(".").ancestors()` yields only "." and "", so every parent
    /// config was silently ignored whenever `--path` was omitted — which is the
    /// common case now that it defaults to the current directory.
    #[test]
    fn a_relative_start_is_made_absolute_so_ancestors_can_be_walked() {
        let absolute = crate::config::absolute(Path::new("."));

        assert!(absolute.is_absolute(), "got {}", absolute.display());
        assert!(
            absolute.ancestors().count() > 2,
            "a relative start must expand to a real chain of parents, got {}",
            absolute.display()
        );
    }
}
