use std::collections::BTreeSet;

pub fn extract(content: &str) -> Vec<String> {
    let stripped = strip_code(content);
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for slug in find_links(&stripped) {
        if seen.insert(slug.clone()) {
            out.push(slug);
        }
    }
    out
}

fn strip_code(content: &str) -> String {
    let no_fences = strip_fenced(content);
    strip_inline(&no_fences)
}

fn strip_fenced(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_fence = false;
    for line in s.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            out.push_str(line);
        }
    }
    out
}

fn strip_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' {
            for c2 in chars.by_ref() {
                if c2 == '`' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn find_links(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let start = i + 2;
            let mut j = start;
            while j + 1 < bytes.len() && !(bytes[j] == b']' && bytes[j + 1] == b']') {
                j += 1;
            }
            if j + 1 < bytes.len() {
                let slug = &s[start..j];
                if is_valid_slug(slug) {
                    out.push(slug.to_string());
                }
                i = j + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn is_valid_slug(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // allow: letters (any case), digits, `-`, `_`, `.`, `/`.
    // disallow: whitespace, control chars, pipe (wiki-style alias syntax).
    s.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/')
    }) && !s.starts_with('/')
        && !s.ends_with('/')
        && !s.contains("//")
}
