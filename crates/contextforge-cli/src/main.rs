mod mcp;

use clap::{Parser, Subcommand};
use contextforge_core::Repository;
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
    let current = std::env::current_dir().map_err(|_| "unable to determine current directory")?;
    let repository = Repository::open(cli.root.as_deref(), &current)?;
    tracing::debug!("repository configuration validated");
    match cli.command {
        Command::Index => {
            let report = contextforge_index::index(&repository)?;
            if cli.json {
                println!("{}", serde_json::to_string(&report)?);
            } else {
                println!(
                    "Indexed {} files: {} added, {} modified, {} unchanged, {} deleted.",
                    report.total_files,
                    report.added,
                    report.modified,
                    report.unchanged,
                    report.deleted
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
