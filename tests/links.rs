use memex::links::extract;

#[test]
fn extracts_valid_slug() {
    assert_eq!(extract("See [[foo]] for details."), vec!["foo"]);
}

#[test]
fn extracts_slug_with_digits_and_dashes() {
    assert_eq!(extract("[[hello-world-2]]"), vec!["hello-world-2"]);
}

#[test]
fn extracts_slug_with_prefix_slash() {
    assert_eq!(
        extract("See [[bp/vision]] and [[phoenix-liveview/context]]."),
        vec!["bp/vision", "phoenix-liveview/context"]
    );
}

#[test]
fn preserves_case_in_slugs() {
    assert_eq!(extract("[[TODOs]] and [[README]]"), vec!["TODOs", "README"]);
}

#[test]
fn dedupes_repeated_slugs_preserving_order() {
    assert_eq!(
        extract("[[alpha]] and [[beta]] then [[alpha]] again."),
        vec!["alpha", "beta"]
    );
}

#[test]
fn ignores_slugs_inside_fenced_code_blocks() {
    let content = "Real [[ref]].\n\n```\n[[ignored]]\n```\n\nMore [[ref]].";
    assert_eq!(extract(content), vec!["ref"]);
}

#[test]
fn ignores_slugs_inside_inline_code() {
    assert_eq!(extract("Try `[[nope]]` but [[yep]] works."), vec!["yep"]);
}

#[test]
fn rejects_invalid_slugs() {
    assert!(extract("[[has space]]").is_empty());
    assert!(extract("[[/leading-slash]]").is_empty());
    assert!(extract("[[trailing-slash/]]").is_empty());
    assert!(extract("[[double//slash]]").is_empty());
    assert!(extract("[[]]").is_empty());
}

#[test]
fn returns_empty_when_no_refs() {
    assert!(extract("Plain text with no refs.").is_empty());
}
