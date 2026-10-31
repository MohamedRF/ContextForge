//! CLI argument adapter for the shared integration preview service.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Harness {
    Claude,
    Codex,
    Cursor,
    Vscode,
}
pub fn preview(
    harness: Harness,
    root: &std::path::Path,
    executable: &std::path::Path,
) -> Result<contextforge_service::integrations::Preview, Box<dyn std::error::Error>> {
    use contextforge_service::integrations as shared;
    let harness = match harness {
        Harness::Claude => shared::Harness::Claude,
        Harness::Codex => shared::Harness::Codex,
        Harness::Cursor => shared::Harness::Cursor,
        Harness::Vscode => shared::Harness::Vscode,
    };
    shared::preview(harness, root, executable).map_err(|error| error as Box<dyn std::error::Error>)
}
