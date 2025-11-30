#[allow(clippy::unwrap_used)]
fn main() {
    uniffi::generate_scaffolding("src/toboggan.udl").unwrap();
}
