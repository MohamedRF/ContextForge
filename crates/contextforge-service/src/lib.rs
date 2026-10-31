//! Shared framework-independent entry points for CLI, MCP and native adapters.
pub mod integrations;
pub mod registry;

pub use contextforge_context as context;
pub use contextforge_core as core;
pub use contextforge_git as git;
pub use contextforge_graph as graph;
pub use contextforge_index as index;
pub use contextforge_parser as parser;

use core::Repository;
use graph::Graph;

pub type ServiceError = Box<dyn std::error::Error + Send + Sync>;

/// Owns orchestration only; individual engines retain their implementations.
pub struct ContextForgeService;

impl ContextForgeService {
    /// Engine capabilities independent of a selected repository or initialized index.
    pub fn diagnostics() -> serde_json::Value {
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "supported_languages": ["rust", "python", "typescript", "javascript"],
            "offline": true
        })
    }

    pub fn load_graph(repository: &Repository) -> Result<Graph, ServiceError> {
        let files = index::snapshot::syntax(repository)?;
        for file in &files {
            let value: serde_json::Value = serde_json::from_str(&file.payload)?;
            if value["path"] != file.path || value["fingerprint"] != file.fingerprint {
                return Err(graph::Error::InvalidSyntax.into());
            }
        }
        Ok(Graph::from_json(
            files.iter().map(|file| file.payload.as_str()),
        )?)
    }

    pub fn pack_context(
        repository: &Repository,
        query: &str,
        budget: usize,
        include_git: bool,
    ) -> Result<context::Package, ServiceError> {
        context::validate(query, budget)?;
        let mut paths = std::collections::BTreeSet::new();
        if include_git {
            let impact = git::analyze(repository, &git::Mode::Unstaged, 0, 100)?;
            for change in impact.direct_changes {
                if let Some(path) = change.new_path.or(change.old_path) {
                    paths.insert(path);
                }
            }
        }
        Ok(context::pack(repository, query, budget, &paths)?)
    }
}
