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
    assert!(extract("[[Foo]]").is_empty());
    assert!(extract("[[1abc]]").is_empty());
    assert!(extract("[[has space]]").is_empty());
    assert!(extract("[[UPPER]]").is_empty());
}

#[test]
fn returns_empty_when_no_refs() {
    assert!(extract("Plain text with no refs.").is_empty());
}
