use anyhow::Result;

use crate::commands::json_escape;
use crate::config::MemexConfig;
use crate::hooks::{self, Event};

pub fn run(cfg: &MemexConfig, file_path: &str, event_str: &str, claude_hook: bool) -> Result<()> {
    let event = Event::parse(event_str)?;
    let set = hooks::load(cfg)?;
    let normalized = hooks::normalize_for_match(file_path, &cfg.project_root);
    let advice = set.advise(event, &normalized, &cfg.project_root);

    if claude_hook {
        emit_claude_hook(event, advice.as_ref());
    } else {
        emit_human(event, advice.as_ref());
    }
    Ok(())
}

fn emit_claude_hook(event: Event, advice: Option<&hooks::HookAdvice>) {
    let Some(advice) = advice else {
        // Nothing matched — emit nothing. Claude Code treats empty stdout as no-op.
        return;
    };
    let msg = match event {
        Event::PreWrite => {
            let mut s = String::new();
            for id in &advice.blueprints {
                s.push_str(&format!(
                    "Read blueprint '{id}' via `memex read {id}` before editing this file, unless it is already in your context. "
                ));
            }
            s
        }
        Event::PostWrite => advice.text.clone().unwrap_or_default(),
    };
    if msg.is_empty() {
        return;
    }
    // Hand-rolled JSON to avoid a dep just for one object. Escape per RFC 8259.
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"{}\",\"additionalContext\":\"{}\"}}}}",
        event.claude_hook_name(),
        json_escape(&msg),
    );
}

fn emit_human(event: Event, advice: Option<&hooks::HookAdvice>) {
    match advice {
        None => println!("(no match)"),
        Some(a) => match event {
            Event::PreWrite => {
                println!("Read: {}", a.blueprints.join(", "));
            }
            Event::PostWrite => {
                if let Some(t) = &a.text {
                    println!("Advice: {t}");
                }
            }
        },
    }
}
