mod intelligence;
mod mcp;
mod setup;

use clap::{Parser, Subcommand};
use contextforge_core::Repository;
use contextforge_service::{index as contextforge_index, parser as contextforge_parser};
use std::{path::PathBuf, process::ExitCode};
use tracing_subscriber::prelude::*;

#[derive(Parser)]
#[command(
    name = "contextforge",
    version,
    about = "Local-first repository intelligence (early development)"
)]
struct Cli {
    /// Explicitly authorize a repository directory (otherwise discover from current directory).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Emit JSON for CLI commands. MCP always uses protocol JSON.
    #[arg(long, global = true)]
    json: bool,
    /// Enable diagnostic tracing on stderr.
    #[arg(short, long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage the project registry shared with ContextForge Desktop.
    Projects {
        #[command(subcommand)]
        action: ProjectCommand,
    },
    /// Preview manual MCP configuration; never modifies harness settings.
    Setup {
        #[arg(value_enum)]
        harness: setup::Harness,
        /// Explicit preview mode (also the default); no files are written.
        #[arg(long)]
        dry_run: bool,
    },
    /// Inspect, clear or rebuild source-linked local structural knowledge.
    Memory {
        #[command(subcommand)]
        action: MemoryCommand,
    },
    /// Pack cited captured source within an estimated token budget (compact JSON).
    Context {
        query: String,
        #[arg(long, default_value_t = 4000, value_parser = clap::value_parser!(u16).range(256..=32000))]
        budget: u16,
        /// Rank matching paths using current unstaged Git changes; requires a Git repository.
        #[arg(long)]
        git: bool,
    },
    /// Analyze Git changed code and bounded likely dependents without executing commands.
    Impact {
        #[arg(long, conflicts_with_all = ["base", "file", "head"])]
        staged: bool,
        /// Compare merge-base(base, head) to head (default HEAD).
        #[arg(long, conflicts_with = "file")]
        base: Option<String>,
        #[arg(long, requires = "base")]
        head: Option<String>,
        /// Explicitly analyze a relative file, even when unchanged.
        #[arg(long, conflicts_with_all = ["base", "head"])]
        file: Option<String>,
        #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(0..=8))]
        depth: u8,
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,
    },
    /// Explore captured syntax dependencies (heuristic calls/imports, bounded traversal).
    Graph {
        selector: String,
        #[arg(long, default_value = "outgoing", value_parser = ["outgoing", "incoming"])]
        direction: String,
        #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=8))]
        depth: u8,
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,
        #[arg(long, default_value_t = 200, value_parser = clap::value_parser!(u16).range(1..=1000))]
        edge_limit: u16,
    },
    /// Find heuristic references and matching unresolved call evidence in the captured index.
    References {
        selector: String,
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,
    },
    /// Search a captured local index; run index explicitly to refresh evidence.
    Search {
        query: String,
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=100))]
        limit: u16,
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u16).range(0..=10000))]
        offset: u16,
    },
    /// Outline current source definitions and unresolved syntax occurrences.
    Analyze {
        /// Eligible source path relative to the authorized root.
        path: PathBuf,
        #[arg(long, default_value_t = 200, value_parser = clap::value_parser!(u16).range(1..=1000))]
        limit: u16,
    },
    /// Transactionally index eligible source files in managed local storage.
    Index,
    /// Create managed configuration without modifying source or ignore files.
    Init,
    /// Show configuration and implementation status without creating files.
    Status,
    /// Check root/configuration access and bounded source discovery.
    Doctor,
    /// List supported UTF-8 source files without creating an index.
    ProjectMap {
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=1000))]
        limit: u16,
    },
    /// Serve implemented tools over MCP stdio (stdout is protocol-only).
    Mcp,
}

#[derive(Subcommand)]
enum MemoryCommand {
    /// Validate stored source fingerprints and show current structural coverage.
    Status,
    /// Clear this repository's managed index rows, keeping source and configuration.
    Clear,
    /// Explicitly capture current sources and reparse all stored structures.
    Rebuild,
}

#[derive(Subcommand)]
enum ProjectCommand {
    List,
    Add { path: PathBuf },
    Remove { id: i64 },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::WARN
    };
    // Third-party protocol diagnostics may contain peer-controlled fields.
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::filter_fn(move |metadata| {
            metadata.target().starts_with("contextforge") && *metadata.level() <= level
        }));
    tracing_subscriber::registry().with(layer).init();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("contextforge: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if let Command::Projects { action } = cli.command {
        let registry = contextforge_service::registry::Registry::user()
            .map_err(|error| error as Box<dyn std::error::Error>)?;
        let value = match action {
            ProjectCommand::List => serde_json::to_value(
                registry
                    .list()
                    .map_err(|error| error as Box<dyn std::error::Error>)?,
            )?,
            ProjectCommand::Add { path } => {
                serde_json::json!({"id": registry.add(&path).map_err(|error| error as Box<dyn std::error::Error>)?})
            }
            ProjectCommand::Remove { id } => {
                registry
                    .remove(id)
                    .map_err(|error| error as Box<dyn std::error::Error>)?;
                serde_json::json!({"unregistered": id, "source_and_index_preserved": true})
            }
        };
        println!("{}", serde_json::to_string(&value)?);
        return Ok(());
    }
    let current = std::env::current_dir().map_err(|_| "unable to determine current directory")?;
    let repository = Repository::open(cli.root.as_deref(), &current)?;
    tracing::debug!("repository configuration validated");
    match cli.command {
        Command::Projects { .. } => {
            unreachable!("project commands are handled before repository discovery")
        }
        Command::Setup {
            harness,
            dry_run: _,
        } => {
            let executable =
                std::env::current_exe().map_err(|_| "unable to locate the running executable")?;
            let preview = setup::preview(harness, repository.root(), &executable)?;
            if cli.json {
                println!("{}", serde_json::to_string(&preview)?);
            } else {
                println!(
                    "Manual configuration for {}\nDestination: {}\n\n{}\n\n{}\n{}",
                    preview.harness,
                    preview.destination,
                    preview.configuration,
                    preview.instructions,
                    preview.verification
                );
            }
        }
        Command::Memory { action } => {
            let value = match action {
                MemoryCommand::Status => {
                    serde_json::to_value(contextforge_index::memory::status(&repository)?)?
                }
                MemoryCommand::Clear => {
                    serde_json::json!({"cleared_files":contextforge_index::memory::clear(&repository)?})
                }
                MemoryCommand::Rebuild => {
                    serde_json::to_value(contextforge_index::memory::rebuild(&repository)?)?
                }
            };
            println!("{}", serde_json::to_string(&value)?);
        }
        Command::Context { query, budget, git } => {
            let result = intelligence::pack_context(&repository, &query, usize::from(budget), git)
                .map_err(|error| error as Box<dyn std::error::Error>)?;
            // Compact JSON is the budgeted format, including when --json is omitted.
            println!("{}", serde_json::to_string(&result)?);
        }
        Command::Impact {
            staged,
            base,
            head,
            file,
            depth,
            limit,
        } => {
            let mode = intelligence::impact_mode(staged, base, head, file)?;
            let result = contextforge_git::analyze(
                &repository,
                &mode,
                usize::from(depth),
                usize::from(limit),
            )?;
            if cli.json {
                println!("{}", serde_json::to_string(&result)?);
            } else {
                for change in &result.direct_changes {
                    println!(
                        "{}\t{}\t{} changed ranges",
                        change.kind,
                        change
                            .new_path
                            .as_deref()
                            .or(change.old_path.as_deref())
                            .unwrap_or("")
                            .escape_debug(),
                        change.hunks.len()
                    );
                }
                for visit in &result.heuristic_dependents {
                    println!(
                        "heuristic dependent\t{}\t{}\tdepth {}",
                        visit.node.path.escape_debug(),
                        visit.node.qualified_name.escape_debug(),
                        visit.depth
                    );
                }
                eprintln!(
                    "{}; truncated: {}; partial syntax: {}",
                    result.uncertainty, result.truncated, result.partial_syntax
                );
            }
        }
        Command::Graph {
            selector,
            direction,
            depth,
            limit,
            edge_limit,
        } => {
            let graph = intelligence::load_graph(&repository)?;
            let direction = if direction == "incoming" {
                contextforge_graph::Direction::Incoming
            } else {
                contextforge_graph::Direction::Outgoing
            };
            let result = graph.traverse(
                &selector,
                direction,
                usize::from(depth),
                usize::from(limit),
                usize::from(edge_limit),
            )?;
            intelligence::print_graph(&result, cli.json)?;
        }
        Command::References { selector, limit } => {
            let graph = intelligence::load_graph(&repository)?;
            let result = graph.find_references(&selector, usize::from(limit))?;
            intelligence::print_graph(&result, cli.json)?;
        }
        Command::Search {
            query,
            limit,
            offset,
        } => {
            let results = contextforge_index::search::search(
                &repository,
                &query,
                usize::from(limit),
                usize::from(offset),
            )?;
            if cli.json {
                println!("{}", serde_json::to_string(&results)?);
            } else {
                for hit in &results.results {
                    println!(
                        "{}:{}-{}\t{}\t{}",
                        hit.path.escape_debug(),
                        hit.span.start_line,
                        hit.span.end_line,
                        hit.symbol.as_deref().unwrap_or("").escape_debug(),
                        hit.method
                    );
                    println!("{}", hit.snippet.escape_debug());
                }
                eprintln!(
                    "{} results; captured snapshot freshness not checked{}",
                    results.results.len(),
                    if results.truncated {
                        "; more results available"
                    } else {
                        ""
                    }
                );
            }
        }
        Command::Analyze { path, limit } => {
            let outline = contextforge_parser::analyze(&repository, &path, usize::from(limit))?;
            if cli.json {
                println!("{}", serde_json::to_string(&outline)?);
            } else {
                println!(
                    "{} ({}, syntax only)",
                    outline.path.escape_debug(),
                    outline.language
                );
                for symbol in outline.symbols {
                    println!(
                        "{}\t{}\t{}:{}-{}",
                        symbol.kind,
                        symbol.name.escape_debug(),
                        outline.path.escape_debug(),
                        symbol.span.start_line,
                        symbol.span.end_line
                    );
                }
                for occurrence in outline.occurrences {
                    println!(
                        "{} (unknown target)\t{}:{}-{}",
                        occurrence.kind,
                        outline.path.escape_debug(),
                        occurrence.span.start_line,
                        occurrence.span.end_line
                    );
                }
                if outline.has_errors {
                    eprintln!("source contains syntax errors; outline may be partial");
                }
                if outline.truncated {
                    eprintln!("outline truncated at configured parser limits");
                }
            }
        }
        Command::Index => {
            let report = contextforge_index::index(&repository)?;
            if cli.json {
                println!("{}", serde_json::to_string(&report)?);
            } else {
                println!(
                    "Indexed {} files: {} added, {} modified, {} unchanged, {} deleted. Parsed {}; partial syntax in {} files.",
                    report.total_files,
                    report.added,
                    report.modified,
                    report.unchanged,
                    report.deleted,
                    report.parsed_files,
                    report.partial_files
                );
            }
        }
        Command::Mcp => mcp::serve(repository).await?,
        Command::Init => {
            repository.init()?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"initialized":true,"config_version":1})
                );
            } else {
                println!(
                    "ContextForge initialized. Recommend adding .contextforge/ to .gitignore."
                );
            }
        }
        Command::Status => {
            let status = repository.status();
            let index = contextforge_index::status(&repository)?;
            if cli.json {
                println!("{}", serde_json::json!({"repository":status,"index":index}));
            } else {
                println!(
                    "Initialized: {}\nGit detected: {}\nIndex: {}",
                    status.initialized, status.git_detected, index.state
                );
            }
        }
        Command::Doctor => {
            let map = repository.project_map(100)?;
            let index = contextforge_index::status(&repository)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"ok":true,"status":repository.status(),"index":index,"source_files_sampled":map.files.len(),"truncated":map.truncated})
                );
            } else {
                println!(
                    "Root and configuration OK. Source files sampled: {}. Index: {} ({} files; freshness not checked).",
                    map.files.len(),
                    index.state,
                    index.files
                );
            }
        }
        Command::ProjectMap { limit } => {
            let map = repository.project_map(usize::from(limit))?;
            if cli.json {
                println!("{}", serde_json::to_string(&map)?);
            } else {
                for file in map.files {
                    println!("{}\t{}\t{} bytes", file.path, file.language, file.bytes);
                }
                if map.truncated {
                    eprintln!("project map truncated at configured discovery limits");
                }
            }
        }
    }
    Ok(())
}
