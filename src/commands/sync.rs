use anyhow::Result;

use crate::config::MemexConfig;
use crate::git;

pub fn run(cfg: &MemexConfig) -> Result<()> {
    for source in &cfg.sources {
        let Some(remote) = &source.remote else {
            println!("{}: skipped (no remote)", source.name);
            continue;
        };
        if !source.mount.exists() {
            if let Some(p) = source.mount.parent() {
                std::fs::create_dir_all(p)?;
            }
            git::clone(remote, &source.mount)?;
            println!("{}: cloned from {remote}", source.name);
            continue;
        }
        match git::pull(&source.mount) {
            Ok(msg) => println!("{}: pull — {msg}", source.name),
            Err(e) => println!("{}: pull failed — {e}", source.name),
        }
        match git::push(&source.mount) {
            Ok(msg) => println!("{}: push — {msg}", source.name),
            Err(e) => println!("{}: push failed — {e}", source.name),
        }
    }
    Ok(())
}
