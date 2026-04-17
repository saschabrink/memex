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
    assert_eq!(hooks::substitute("formatted: ${0}", &caps), "formatted: lib/foo.ex");
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
    assert_eq!(set.pre_write[0].blueprints, vec!["phoenix-liveview/liveview"]);
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
    assert_eq!(
        set.pre_write[0].source.as_deref(),
        Some("testsource")
    );
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
    assert!(err.contains("pre-write") && err.contains("text"), "got: {err}");
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
    assert!(err.contains("pre-write") && err.contains("needs"), "got: {err}");
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
    assert!(err.contains("post-write") && err.contains("text"), "got: {err}");
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
    let pre = set
        .advise(Event::PreWrite, "x", &env.project_dir)
        .unwrap();
    assert_eq!(pre.blueprints, vec!["bp"]);
    assert!(pre.text.is_none());

    let post = set
        .advise(Event::PostWrite, "x", &env.project_dir)
        .unwrap();
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
    let fired = set.advise(
        Event::PostWrite,
        "lib/foo.ex",
        &env.project_dir,
    );
    assert!(fired.is_some());
    assert_eq!(
        fired.unwrap().text.as_deref(),
        Some("write test/foo_test.exs")
    );

    // Create the target; now the hook should NOT fire.
    write_file(&env.project_dir.join("test/foo_test.exs"), "");
    let silent = set.advise(
        Event::PostWrite,
        "lib/foo.ex",
        &env.project_dir,
    );
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
    let fired = set.advise(
        Event::PostWrite,
        "lib/foo.ex",
        &env.project_dir,
    );
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

// ---------- TestEnv helper ----------

impl TestEnv {
    /// Alias for the common test fixture used throughout this file.
    pub fn default_env() -> Self {
        common::create_env()
    }
}
