use std::fs;

use jiff::civil::DateTime;
use toboggan_core::{Content, Slide, SlideKind, Style, Talk};

fn main() -> anyhow::Result<()> {
    let talk = Talk {
        title: Content::Text {
            text: "Peut-on RIIR de tout ?".to_string(),
        },
        date: DateTime::new(2025, 11, 13, 9, 10, 0, 0)?,
        slides: vec![
            Slide {
                kind: SlideKind::Cover,
                body: Content::Empty,
                ..slide("Peut-on RIIR de tout ?", "")
            },
            slide(
                "Introduction",
                r#"
<p>
<strong>RIIR</strong> : "Have you considered Rewriting It In Rust?"
</p>
<p>
Une question qui fait sourire... mais qui cache une réalité : Rust gagne du terrain partout.
</p>
                "#,
            ),
            Slide {
                kind: SlideKind::Part,
                body: Content::Empty,
                ..slide("1. Les Success Stories du RIIR", "")
            },
            slide(
                "Des réécritures qui ont fait leurs preuves",
                r#"

- **ripgrep** (`rg`) : grep réécrit en Rust
  - 10x plus rapide que grep classique
  - Recherche récursive native
  - Support Unicode complet

- **fd** : find réécrit en Rust
  - Interface plus intuitive
  - Performances supérieures
  - Respect des .gitignore par défaut

- **Fish Shell** : Shell moderne
  - Autocomplétion intelligente
  - Sécurité mémoire
  - Configuration simple
                "#,
            ),
        ],
    };

    let toml = toml::to_string_pretty(&talk)?;
    fs::write("./talk.toml", toml)?;

    Ok(())
}

fn slide(title: &str, body: &str) -> Slide {
    Slide {
        kind: SlideKind::Standard,
        style: Style::default(),
        title: Content::Text {
            text: title.to_string(),
        },
        body: Content::Html {
            raw: body.to_string(),
            alt: None,
        },
        notes: Content::Empty,
    }
}
