#![allow(clippy::print_stderr)]

use std::path::Path;

fn main() {
    let dist_path = Path::new("../toboggan-web/dist");

    // Verify that the dist folder exists
    if !dist_path.exists() {
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        eprintln!("ERROR: Web dist folder not found!");
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        eprintln!();
        eprintln!("The toboggan-server requires the web frontend to be built first.");
        eprintln!();
        eprintln!("Please run one of the following commands:");
        eprintln!("  mise build:web    # Build just the web frontend");
        eprintln!("  mise build        # Build all components in correct order");
        eprintln!();
        eprintln!("The web frontend will be statically embedded in the server binary.");
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        panic!("Build failed: missing web dist folder");
    }

    // Tell cargo to rerun if the dist folder changes
    println!("cargo:rerun-if-changed=../toboggan-web/dist");

    // Also watch for changes to the web source files
    println!("cargo:rerun-if-changed=../toboggan-web/package.json");
    println!("cargo:rerun-if-changed=../toboggan-web/vite.config.ts");
}
