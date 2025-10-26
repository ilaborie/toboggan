use std::path::Path;

fn main() {
    // Check that the web frontend dist folder exists and is not empty
    let web_dist_path = Path::new("../toboggan-web/dist");

    if !web_dist_path.exists() {
        panic!(
            "\n\n❌ ERROR: Web frontend dist folder not found!\n\
             \n\
             The toboggan-server embeds web assets at compile time using RustEmbed.\n\
             You must build the web frontend BEFORE compiling the server.\n\
             \n\
             Please run:\n\
             \n\
             1. Build web frontend:\n\
                cd toboggan-web && npm run build && cd ..\n\
             \n\
             2. Then build the server:\n\
                cargo build -p toboggan-server\n\
             \n\
             Or use the mise tasks which handle the correct order:\n\
                mise build:rust  # Builds web first, then Rust\n\
                mise serve       # Ensures web is built before running server\n\
             \n"
        );
    }

    // Check that the dist folder contains the essential files
    let index_html = web_dist_path.join("index.html");
    if !index_html.exists() {
        panic!(
            "\n\n❌ ERROR: Web frontend dist folder is incomplete!\n\
             \n\
             The dist/index.html file is missing. The web frontend may not have\n\
             been built correctly.\n\
             \n\
             Please rebuild the web frontend:\n\
                cd toboggan-web && npm run build && cd ..\n\
             \n"
        );
    }

    // Tell Cargo to rerun this build script if the dist folder changes
    println!("cargo:rerun-if-changed=../toboggan-web/dist");

    // Also rerun if any files in the dist folder change
    // This ensures the server is rebuilt when the web frontend is rebuilt
    if web_dist_path.exists() {
        for entry in std::fs::read_dir(web_dist_path).expect("Failed to read dist directory") {
            if let Ok(entry) = entry {
                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
        }
    }

    println!("cargo:warning=✅ Web frontend dist folder found and valid");
}
