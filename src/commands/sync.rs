use anyhow::Result;

use crate::config::MemexConfig;
use crate::git;

/// Sync remote-backed sources. Uses `git pull --ff-only` — never merges or
/// rebases. Divergent branches are reported as errors (non-fatal for the
/// overall sync). For writable sources, pushes local commits after pulling.
pub fn run(cfg: &MemexConfig) -> Result<()> {
    let mut any_remote = false;
    for source in &cfg.sources {
        let Some(remote) = &source.remote else {
            continue;
        };
        any_remote = true;

        // Clone if the mount doesn't exist yet.
        if !source.mount.exists() {
            if let Some(p) = source.mount.parent() {
                std::fs::create_dir_all(p)?;
            }
            match git::clone(remote, &source.mount) {
                Ok(()) => println!("{}: cloned", source.name),
                Err(e) => println!("{}: clone failed — {e}", source.name),
            }
            continue;
        }

        // If the configured remote URL changed, update the local clone.
        match git::get_remote_url(&source.mount, "origin") {
            Ok(Some(current)) if current != *remote => {
                match git::set_remote_url(&source.mount, "origin", remote) {
                    Ok(()) => println!("{}: remote updated → {remote}", source.name),
                    Err(e) => println!("{}: failed to update remote — {e}", source.name),
                }
            }
            _ => {}
        }

        // Fast-forward-only pull.
        match git::pull_ff_only(&source.mount) {
            Ok(msg) => println!("{}: {msg}", source.name),
            Err(e) => println!("{}: pull failed — {e}", source.name),
        }

        // Push for writable remotes only. Readonly sources never push.
        if !source.readonly {
            match git::push(&source.mount) {
                Ok(_) => {}
                Err(e) => println!("{}: push failed — {e}", source.name),
            }
        }
    }
    if !any_remote {
        println!("No remote-backed sources configured.");
    }
    Ok(())
}
