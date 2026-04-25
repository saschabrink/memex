mod common;

use common::{mk_tmp, TestEnv};
use memex::config;
use memex::hooks::{self, Event};

fn write_file(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

// ---------- substitution ----------

#[test]
fn substitute_replaces_group_references() {
    let re = fancy_regex::Regex::new(r"^lib/(.+)\.ex$").unwrap();
    let caps = re.captures("lib/foo/bar.ex").unwrap().unwrap();
    assert_eq!(
        hooks::substitute("test/${1}_test.exs", &caps),
        "test/foo/bar_test.exs"
    );
}

#[test]
fn substitute_handles_group_zero_as_full_match() {
    let re = fancy_regex::Regex::new(r"^lib/(.+)\.ex$").unwrap();
    let caps = re.captures("lib/foo.ex").unwrap().unwrap();
    assert_eq!(
        hooks::substitute("formatted: ${0}", &caps),
        "formatted: lib/foo.ex"
    );
}

#[test]
fn substitute_leaves_non_numeric_placeholders_alone() {
    let re = fancy_regex::Regex::new(r"^(.+)$").unwrap();
    let caps = re.captures("x").unwrap().unwrap();
    assert_eq!(hooks::substitute("${bogus}", &caps), "${bogus}");
}

// ---------- loading ----------

#[test]
fn load_returns_empty_when_no_hooks_file_present() {
    let env = TestEnv::default_env();
    let set = hooks::load(&env.cfg).unwrap();
    assert!(set.pre_write.is_empty());
    assert!(set.post_write.is_empty());
}

#[test]
fn load_reads_project_level_hooks_toml() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "_live\\.ex$"
blueprint = "phoenix-liveview/liveview"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();
    assert_eq!(set.pre_write.len(), 1);
    assert_eq!(
        set.pre_write[0].blueprints,
        vec!["phoenix-liveview/liveview"]
    );
    assert!(set.pre_write[0].source.is_none()); // project-level
}

#[test]
fn load_reads_source_level_hooks_toml() {
    let env = TestEnv::default_env();
    write_file(
        &env.source_mount.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "ctx\\.ex$"
blueprints = ["a", "b"]
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();
    assert_eq!(set.pre_write.len(), 1);
    assert_eq!(set.pre_write[0].blueprints, vec!["a", "b"]);
    assert_eq!(set.pre_write[0].source.as_deref(), Some("testsource"));
}

#[test]
fn load_merges_project_then_source_order() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "^a$"
blueprint = "proj"
"#,
    );
    write_file(
        &env.source_mount.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "^a$"
blueprint = "source"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();
    assert_eq!(set.pre_write.len(), 2);
    assert_eq!(set.pre_write[0].blueprints, vec!["proj"]);
    assert_eq!(set.pre_write[1].blueprints, vec!["source"]);
}

#[test]
fn load_errors_on_pre_write_with_text() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "x"
blueprint = "foo"
text = "nope"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(
        err.contains("pre-write") && err.contains("text"),
        "got: {err}"
    );
}

#[test]
fn load_errors_on_post_write_with_blueprint() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[post-write]]
pattern = "x"
blueprint = "nope"
text = "text"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(
        err.contains("post-write") && err.contains("blueprint"),
        "got: {err}"
    );
}

#[test]
fn load_errors_on_both_blueprint_and_blueprints() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "x"
blueprint = "a"
blueprints = ["b"]
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(
        err.contains("both") || err.contains("pick one"),
        "got: {err}"
    );
}

#[test]
fn load_errors_on_pre_write_without_blueprint() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "x"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(
        err.contains("pre-write") && err.contains("needs"),
        "got: {err}"
    );
}

#[test]
fn load_errors_on_post_write_without_text() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[post-write]]
pattern = "x"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(
        err.contains("post-write") && err.contains("text"),
        "got: {err}"
    );
}

#[test]
fn load_errors_on_invalid_regex() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "*"
blueprint = "foo"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(err.contains("regex") || err.contains("*"), "got: {err}");
}

#[test]
fn load_accepts_blueprint_as_string_or_list() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "a"
blueprint = "single"

[[pre-write]]
pattern = "b"
blueprints = ["one", "two"]
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();
    assert_eq!(set.pre_write[0].blueprints, vec!["single"]);
    assert_eq!(set.pre_write[1].blueprints, vec!["one", "two"]);
}

// ---------- advise (first match wins) ----------

#[test]
fn advise_returns_first_matching_pre_write_hook() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "lib/platform/([^/]+)/\\1\\.ex$"
blueprint = "context"

[[pre-write]]
pattern = "lib/platform/[^/]+/[^/]+\\.ex$"
blueprint = "schema"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();

    // Context/context.ex — matches the first (backref) hook.
    let a = set
        .advise(
            Event::PreWrite,
            "lib/platform/articles/articles.ex",
            &env.project_dir,
        )
        .unwrap();
    assert_eq!(a.blueprints, vec!["context"]);

    // Context/other.ex — doesn't match the backref, falls through to schema.
    let b = set
        .advise(
            Event::PreWrite,
            "lib/platform/articles/article.ex",
            &env.project_dir,
        )
        .unwrap();
    assert_eq!(b.blueprints, vec!["schema"]);
}

#[test]
fn advise_returns_none_when_nothing_matches() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "_live\\.ex$"
blueprint = "liveview"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();
    assert!(set
        .advise(Event::PreWrite, "lib/foo.ex", &env.project_dir)
        .is_none());
}

#[test]
fn advise_separates_pre_and_post_write_buckets() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "x"
blueprint = "bp"

[[post-write]]
pattern = "x"
text = "advice"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();
    let pre = set.advise(Event::PreWrite, "x", &env.project_dir).unwrap();
    assert_eq!(pre.blueprints, vec!["bp"]);
    assert!(pre.text.is_none());

    let post = set.advise(Event::PostWrite, "x", &env.project_dir).unwrap();
    assert_eq!(post.text.as_deref(), Some("advice"));
    assert!(post.blueprints.is_empty());
}

// ---------- conditions ----------

#[test]
fn when_file_missing_fires_only_if_target_absent() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[post-write]]
pattern = "^lib/(.+)\\.ex$"
text = "write test/${1}_test.exs"
when_file_missing = "test/${1}_test.exs"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();

    // Target absent → fires.
    let fired = set.advise(Event::PostWrite, "lib/foo.ex", &env.project_dir);
    assert!(fired.is_some());
    assert_eq!(
        fired.unwrap().text.as_deref(),
        Some("write test/foo_test.exs")
    );

    // Create the target; now the hook should NOT fire.
    write_file(&env.project_dir.join("test/foo_test.exs"), "");
    let silent = set.advise(Event::PostWrite, "lib/foo.ex", &env.project_dir);
    assert!(silent.is_none());
}

#[test]
fn when_file_exists_fires_only_if_target_present() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[post-write]]
pattern = "^lib/(.+)\\.ex$"
text = "target is there"
when_file_exists = "lib/${1}_companion.ex"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();

    // Companion absent → no fire.
    assert!(set
        .advise(Event::PostWrite, "lib/foo.ex", &env.project_dir)
        .is_none());

    // Companion present → fires.
    write_file(&env.project_dir.join("lib/foo_companion.ex"), "");
    let fired = set.advise(Event::PostWrite, "lib/foo.ex", &env.project_dir);
    assert!(fired.is_some());
}

// ---------- normalization ----------

#[test]
fn normalize_for_match_strips_project_root_for_absolute_paths() {
    let env = TestEnv::default_env();
    let abs = env.project_dir.join("lib/foo.ex");
    let s = hooks::normalize_for_match(&abs.to_string_lossy(), &env.project_dir);
    assert_eq!(s, "lib/foo.ex");
}

#[test]
fn normalize_for_match_preserves_relative_paths() {
    let env = TestEnv::default_env();
    assert_eq!(
        hooks::normalize_for_match("lib/foo.ex", &env.project_dir),
        "lib/foo.ex"
    );
}

// ---------- readonly source ----------

#[test]
fn readonly_source_loads_with_flag() {
    let tmp = mk_tmp("memex-ro-");
    std::fs::write(
        tmp.join("memex.toml"),
        r#"project_name = "p"

[normal]
mount = "docs"

[frozen]
mount = "vendor"
readonly = true
"#,
    )
    .unwrap();
    std::fs::create_dir_all(tmp.join("docs")).unwrap();
    std::fs::create_dir_all(tmp.join("vendor")).unwrap();

    let cfg = config::load(&tmp, None).unwrap();
    assert!(!cfg.source_by_name("normal").unwrap().readonly);
    assert!(cfg.source_by_name("frozen").unwrap().readonly);
}

// ---------- content_pattern ----------

#[test]
fn content_pattern_matches_when_file_contains_marker() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "lib/.*\\.ex$"
content_pattern = "(?m)^\\s*use Ecto\\.Schema"
blueprint = "ecto-schema"
"#,
    );
    write_file(
        &env.project_dir.join("lib/foo.ex"),
        "defmodule Foo do\n  use Ecto.Schema\nend\n",
    );
    write_file(
        &env.project_dir.join("lib/bar.ex"),
        "defmodule Bar do\n  # no schema here\nend\n",
    );

    let set = hooks::load(&env.cfg).unwrap();
    let hit = set
        .advise(Event::PreWrite, "lib/foo.ex", &env.project_dir)
        .unwrap();
    assert_eq!(hit.blueprints, vec!["ecto-schema"]);

    assert!(set
        .advise(Event::PreWrite, "lib/bar.ex", &env.project_dir)
        .is_none());
}

#[test]
fn content_pattern_alone_without_path_pattern_is_valid() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
content_pattern = "Req\\.Test"
blueprint = "req-testing"
"#,
    );
    write_file(
        &env.project_dir.join("test/some_test.exs"),
        "setup do\n  Req.Test.stub(...)\nend\n",
    );

    let set = hooks::load(&env.cfg).unwrap();
    let hit = set
        .advise(Event::PreWrite, "test/some_test.exs", &env.project_dir)
        .unwrap();
    assert_eq!(hit.blueprints, vec!["req-testing"]);
}

#[test]
fn content_pattern_does_not_match_when_file_missing() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "lib/.*\\.ex$"
content_pattern = "anything"
blueprint = "x"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();
    assert!(set
        .advise(Event::PreWrite, "lib/nonexistent.ex", &env.project_dir)
        .is_none());
}

#[test]
fn content_pattern_skips_oversize_files() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
content_pattern = "marker"
blueprint = "x"
"#,
    );
    // Build a file just over the 1 MB cap, with the marker at the top.
    let big = format!(
        "marker\n{}",
        "a".repeat(memex::hooks::MAX_CONTENT_BYTES as usize)
    );
    write_file(&env.project_dir.join("big.txt"), &big);

    let set = hooks::load(&env.cfg).unwrap();
    assert!(set
        .advise(Event::PreWrite, "big.txt", &env.project_dir)
        .is_none());
}

#[test]
fn load_errors_when_neither_pattern_nor_content_pattern_set() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
blueprint = "x"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(
        err.contains("pattern") && err.contains("content_pattern"),
        "got: {err}"
    );
}

#[test]
fn load_errors_on_content_pattern_with_when_file_missing() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "lib/.*"
content_pattern = "foo"
when_file_missing = "bar"
blueprint = "x"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(err.contains("when_file_missing"), "got: {err}");
}

#[test]
fn load_errors_on_invalid_content_pattern_regex() {
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "lib/.*"
content_pattern = "*"
blueprint = "x"
"#,
    );
    let err = format!("{:#}", hooks::load(&env.cfg).unwrap_err());
    assert!(err.contains("content_pattern"), "got: {err}");
}

// ---------- hook_base ----------

/// Build a TestEnv where the `phoenix-docs` source has `hook_base = "backend"`.
fn env_with_hook_base(hook_base: &str) -> TestEnv {
    let tmp_dir = mk_tmp("memex-hookbase-");
    let project_dir = tmp_dir.join("project");
    std::fs::create_dir_all(&project_dir).unwrap();
    common::git_init(&project_dir);

    // Source mount lives inside the project so hooks.toml is easy to place.
    let source_mount = project_dir.join("shared-docs");
    std::fs::create_dir_all(&source_mount).unwrap();

    std::fs::write(
        project_dir.join("memex.toml"),
        format!(
            r#"project_name = "testproject"

[shared-docs]
mount = "shared-docs"
hook_base = "{hook_base}"
"#
        ),
    )
    .unwrap();

    let cfg = config::load(&project_dir, None).unwrap();
    cfg.ensure_initialized().unwrap();

    TestEnv {
        tmp_dir,
        project_dir,
        source_mount,
        cfg,
    }
}

#[test]
fn hook_base_scopes_hook_to_subdirectory() {
    let env = env_with_hook_base("backend");
    write_file(
        &env.source_mount.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "lib/.*\\.ex$"
blueprint = "phoenix-rules"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();

    // File under the base → fires.
    let hit = set.advise(Event::PreWrite, "backend/lib/foo.ex", &env.project_dir);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().blueprints, vec!["phoenix-rules"]);

    // File outside the base → no match.
    assert!(set
        .advise(Event::PreWrite, "mobile/lib/foo.ex", &env.project_dir)
        .is_none());

    // File at project root (no base prefix) → no match.
    assert!(set
        .advise(Event::PreWrite, "lib/foo.ex", &env.project_dir)
        .is_none());
}

#[test]
fn hook_base_strips_prefix_for_pattern_captures() {
    let env = env_with_hook_base("backend");
    write_file(
        &env.source_mount.join("hooks.toml"),
        r#"
[[post-write]]
pattern = "^lib/(.+)\\.ex$"
text = "write test/${1}_test.exs"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();

    let advice = set
        .advise(
            Event::PostWrite,
            "backend/lib/accounts.ex",
            &env.project_dir,
        )
        .unwrap();
    // ${1} should be the base-relative capture, not include "backend/".
    assert_eq!(advice.text.as_deref(), Some("write test/accounts_test.exs"));
}

#[test]
fn hook_base_when_file_missing_resolved_under_base() {
    let env = env_with_hook_base("backend");
    write_file(
        &env.source_mount.join("hooks.toml"),
        r#"
[[post-write]]
pattern = "^lib/(.+)\\.ex$"
text = "create test/${1}_test.exs"
when_file_missing = "test/${1}_test.exs"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();

    // Test file absent under backend/ → fires.
    let fired = set.advise(Event::PostWrite, "backend/lib/orders.ex", &env.project_dir);
    assert!(fired.is_some());

    // Create the test file under backend/ → hook should no longer fire.
    write_file(&env.project_dir.join("backend/test/orders_test.exs"), "");
    let silent = set.advise(Event::PostWrite, "backend/lib/orders.ex", &env.project_dir);
    assert!(silent.is_none());
}

#[test]
fn hook_base_without_hook_base_still_matches_from_root() {
    // Verify existing behavior is unchanged when hook_base is not set.
    let env = TestEnv::default_env();
    write_file(
        &env.project_dir.join("hooks.toml"),
        r#"
[[pre-write]]
pattern = "lib/.*\\.ex$"
blueprint = "elixir-rules"
"#,
    );
    let set = hooks::load(&env.cfg).unwrap();

    let hit = set.advise(Event::PreWrite, "lib/foo.ex", &env.project_dir);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().blueprints, vec!["elixir-rules"]);
}

#[test]
fn hook_base_parsed_from_memex_toml() {
    let tmp = mk_tmp("memex-hookbase-cfg-");
    std::fs::write(
        tmp.join("memex.toml"),
        r#"project_name = "p"

[phoenix-docs]
mount = "docs"
hook_base = "backend"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(tmp.join("docs")).unwrap();

    let cfg = config::load(&tmp, None).unwrap();
    let src = cfg.source_by_name("phoenix-docs").unwrap();
    assert_eq!(src.hook_base.as_deref(), Some("backend"));
}

#[test]
fn hook_base_absent_when_not_configured() {
    let tmp = mk_tmp("memex-hookbase-none-");
    std::fs::write(
        tmp.join("memex.toml"),
        r#"project_name = "p"

[docs]
mount = "docs"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(tmp.join("docs")).unwrap();

    let cfg = config::load(&tmp, None).unwrap();
    assert!(cfg.source_by_name("docs").unwrap().hook_base.is_none());
}

// ---------- TestEnv helper ----------

impl TestEnv {
    /// Alias for the common test fixture used throughout this file.
    pub fn default_env() -> Self {
        common::create_env()
    }
}
