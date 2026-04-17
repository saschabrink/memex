use anyhow::Result;

use crate::config::MemexConfig;
use crate::git;

pub fn run(cfg: &MemexConfig) -> Result<()> {
    for source in &cfg.sources {
        let Some(remote) = &source.remote else {
            println!("{}: skipped (no remote)", source.name);
            continue;
        };
        if !source.path.exists() {
            git::clone(remote, &source.path)?;
            println!("{}: cloned from {remote}", source.name);
            continue;
        }
        match git::pull(&source.path) {
            Ok(msg) => println!("{}: pull — {msg}", source.name),
            Err(e) => println!("{}: pull failed — {e}", source.name),
        }
        match git::push(&source.path) {
            Ok(msg) => println!("{}: push — {msg}", source.name),
            Err(e) => println!("{}: push failed — {e}", source.name),
        }
    }
    Ok(())
}
