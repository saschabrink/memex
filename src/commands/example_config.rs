//! `memex example-config` — prints a fully annotated memex.toml example.
//!
//! The example is embedded at compile time from `examples/memex.example.toml`,
//! so it matches the binary version exactly and works offline.
//!
//! Intended use: `memex example-config > memex.toml` to bootstrap a project,
//! or `memex example-config | less` to look up an option without leaving the
//! terminal. Agents should invoke this before editing an unfamiliar
//! `memex.toml`.

use anyhow::Result;

const EXAMPLE: &str = include_str!("../../examples/memex.example.toml");

pub fn run() -> Result<()> {
    print!("{EXAMPLE}");
    Ok(())
}
