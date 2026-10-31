use contextforge_core::Repository;
use contextforge_graph::{Graph, GraphResult};

pub(crate) fn pack_context(
    repository: &Repository,
    query: &str,
    budget: usize,
    include_git: bool,
) -> Result<contextforge_context::Package, Box<dyn std::error::Error + Send + Sync>> {
    contextforge_service::ContextForgeService::pack_context(repository, query, budget, include_git)
}

pub(crate) fn impact_mode(
    staged: bool,
    base: Option<String>,
    head: Option<String>,
    file: Option<String>,
) -> Result<contextforge_git::Mode, contextforge_git::Error> {
    if usize::from(staged) + usize::from(base.is_some()) + usize::from(file.is_some()) > 1
        || (head.is_some() && base.is_none())
    {
        return Err(contextforge_git::Error::InvalidQuery);
    }
    Ok(if staged {
        contextforge_git::Mode::Staged
    } else if let Some(base) = base {
        contextforge_git::Mode::Base {
            base,
            head: head.unwrap_or_else(|| "HEAD".into()),
        }
    } else if let Some(file) = file {
        contextforge_git::Mode::File(file)
    } else {
        contextforge_git::Mode::Unstaged
    })
}

pub(crate) fn load_graph(repository: &Repository) -> Result<Graph, Box<dyn std::error::Error>> {
    contextforge_service::ContextForgeService::load_graph(repository)
        .map_err(|error| error as Box<dyn std::error::Error>)
}

pub(crate) fn print_graph(result: &GraphResult, json: bool) -> Result<(), serde_json::Error> {
    if json {
        println!("{}", serde_json::to_string(result)?);
    } else {
        for visit in &result.nodes {
            println!(
                "{}\t{}\t{:?}\tdepth {}",
                visit.node.path.escape_debug(),
                visit.node.qualified_name.escape_debug(),
                visit.status,
                visit.depth
            );
        }
        eprintln!(
            "{} relation evidence records; snapshot freshness not checked; partial syntax: {}; truncated: {}",
            result.edges.len(),
            result.partial_syntax,
            result.truncated
        );
    }
    Ok(())
}
