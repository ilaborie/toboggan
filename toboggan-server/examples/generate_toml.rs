use std::fs;

use toboggan_core::{Content, Date, Slide, Talk};

fn main() -> anyhow::Result<()> {
    let talk = Talk::new("Peut-on RIIR de tout ?")
        .with_date(Date::new(2025, 11, 13))
        .add_slide(Slide::cover("Peut-on RIIR de tout ?"))
        .add_slide(slide(
            "Introduction",
            r#"
<p>
<strong>RIIR</strong> : "Have you considered Rewriting It In Rust?"
</p>
<p>
Une question qui fait sourire... mais qui cache une réalité : Rust gagne du terrain partout.
</p>
                "#,
        ))
        .add_slide(Slide::part("1. Les Success Stories du RIIR"))
        .add_slide(slide(
            "Des réécritures qui ont fait leurs preuves",
            r"

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
                ",
        ));

    let toml = toml::to_string_pretty(&talk)?;
    fs::write("./talk.toml", toml)?;

    Ok(())
}

fn slide(title: &str, body: &str) -> Slide {
    Slide::new(title).with_body(Content::html(body))
}
