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
    #[command(
        about = "Print hook advice for a file (blueprints to read / post-write text).",
        long_about = "Looks up matching hooks in hooks.toml files (project-level + per-source). \
                      With --claude-hook, emits the JSON expected by Claude Code's hook system."
    )]
    HookAdvice {
        file: String,
        #[arg(long, default_value = "pre-write", help = "pre-write | post-write")]
        event: String,
        #[arg(long, help = "Emit Claude Code hook JSON instead of human output.")]
        claude_hook: bool,
    },
    #[command(
        about = "Print generic usage instructions for LLM agents.",
        long_about = "Outputs a project-agnostic brief on how to use memex tools. \
                      Intended for CLAUDE.md inclusion or a Claude Code SessionStart hook. \
                      Does not require a memex.toml."
    )]
    AgentInstructions {
        #[arg(long, help = "Emit Claude Code SessionStart hook JSON instead of plain markdown.")]
        claude_hook: bool,
    },
    #[command(
        about = "Print a fully annotated memex.toml example.",
        long_about = "Prints the annotated memex.toml example embedded in this binary. \
                      Use to bootstrap a project (`memex example-config > memex.toml`) \
                      or to look up an option without leaving the terminal. \
                      Does not require a memex.toml."
    )]
    ExampleConfig,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Commands that do not require a memex.toml.
    if let Command::AgentInstructions { claude_hook } = &cli.command {
        return commands::agent_instructions::run(*claude_hook);
    }
    if let Command::ExampleConfig = &cli.command {
        return commands::example_config::run();
    }

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
        Command::HookAdvice {
            file,
            event,
            claude_hook,
        } => commands::hook_advice::run(&cfg, &file, &event, claude_hook),
        Command::AgentInstructions { .. } => unreachable!("handled above"),
        Command::ExampleConfig => unreachable!("handled above"),
    }
}
