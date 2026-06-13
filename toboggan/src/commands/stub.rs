/// Returns a clear "not yet implemented" error for subcommands whose backing
/// crate has not landed yet.
pub(crate) fn coming_soon(feature: &str) -> miette::Result<()> {
    Err(miette::miette!(
        "`toboggan {feature}` is not implemented yet — coming in a later milestone"
    ))
}
