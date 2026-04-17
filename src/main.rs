use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use memex::{commands, config};

#[derive(Parser)]
#[command(name = "memex", version, about = "Multi-source blueprint knowledge base.")]
struct Cli {
    #[arg(long, global = true, help = "Override memex.toml path")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Semantic search across blueprints.")]
    Search {
        query: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
    #[command(about = "List all blueprints.")]
    List,
    #[command(about = "Read full content of a blueprint.")]
    Read { id: String },
    #[command(about = "Create or overwrite a blueprint.")]
    Write { id: String, content: String },
    #[command(about = "Partial edit: replace old_string with new_string.")]
    Edit {
        id: String,
        old_string: String,
        new_string: String,
    },
    #[command(about = "Delete a blueprint.")]
    Delete { id: String },
    #[command(about = "Move/rename a blueprint.")]
    Move { old_id: String, new_id: String },
    #[command(about = "Rebuild vector index from files.")]
    RebuildIndex,
    #[command(about = "List git history for a blueprint.")]
    Versions { id: String },
    #[command(about = "Show diff introduced by a specific commit.")]
    Diff { id: String, hash: String },
    #[command(about = "Find blueprints with broken [[slug]] references.")]
    BrokenRefs,
    #[command(about = "Sync sources that have a remote (clone/pull/push).")]
    Sync,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;
    let cfg = config::load(&cwd, cli.config.as_deref())?;
    cfg.ensure_initialized()?;

    match cli.command {
        Command::List => commands::list::run(&cfg),
        Command::Read { id } => commands::read::run(&cfg, &id),
        Command::Search { query, limit } => commands::search::run(&cfg, &query, limit),
        Command::Write { id, content } => commands::write::run(&cfg, &id, &content),
        Command::Edit {
            id,
            old_string,
            new_string,
        } => commands::edit::run(&cfg, &id, &old_string, &new_string),
        Command::Delete { id } => commands::delete::run(&cfg, &id),
        Command::Move { old_id, new_id } => commands::move_::run(&cfg, &old_id, &new_id),
        Command::RebuildIndex => commands::rebuild_index::run(&cfg),
        Command::Versions { id } => commands::versions::run(&cfg, &id),
        Command::Diff { id, hash } => commands::diff::run(&cfg, &id, &hash),
        Command::BrokenRefs => commands::broken_refs::run(&cfg),
        Command::Sync => commands::sync::run(&cfg),
    }
}
