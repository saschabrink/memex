mod common;

use std::path::PathBuf;

use memex::config;

use common::{create_env, mk_tmp};

// ---------- discover ----------

#[test]
fn discover_finds_memex_toml_in_given_directory() {
    let dir = mk_tmp("memex-discover-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\n",
    )
    .unwrap();
    let p = config::discover(&dir).unwrap();
    assert_eq!(p, dir.join("memex.toml"));
}

#[test]
fn discover_does_not_look_in_config_subdir() {
    let dir = mk_tmp("memex-discover-");
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::write(
        dir.join("config").join("memex.toml"),
        "project_name = \"p\"\n\n[s]\n",
    )
    .unwrap();
    // Not at the root of `dir`, so discover must fail.
    assert!(config::discover(&dir).is_err());
}

#[test]
fn discover_does_not_look_in_dot_memex() {
    let dir = mk_tmp("memex-discover-");
    std::fs::create_dir_all(dir.join(".memex")).unwrap();
    std::fs::write(
        dir.join(".memex").join("memex.toml"),
        "project_name = \"p\"\n\n[s]\n",
    )
    .unwrap();
    assert!(config::discover(&dir).is_err());
}

#[test]
fn discover_errors_when_no_config_found() {
    let dir = mk_tmp("memex-discover-");
    let err = config::discover(&dir).unwrap_err().to_string();
    assert!(err.contains("No memex.toml found"), "got: {err}");
}

// ---------- load ----------

#[test]
fn load_parses_project_name() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"acme\"\n\n[s]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.project_name, "acme");
}

#[test]
fn load_errors_on_empty_project_name() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"\"\n\n[s]\n",
    )
    .unwrap();
    assert!(config::load(&dir, None).is_err());
}

#[test]
fn load_errors_when_project_name_missing() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "[s]\nmount = \".\"\n",
    )
    .unwrap();
    assert!(config::load(&dir, None).is_err());
}

#[test]
fn load_resolves_project_root_relative_to_config() {
    let dir = mk_tmp("memex-parse-");
    let cfg_dir = dir.join("sub");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("memex.toml"),
        "project_name = \"p\"\nroot = \"..\"\n\n[s]\n",
    )
    .unwrap();
    let cfg = config::load(&cfg_dir, None).unwrap();
    // project_root = cfg_dir/.. = dir
    assert_eq!(
        cfg.project_root.canonicalize().unwrap(),
        dir.canonicalize().unwrap()
    );
}

#[test]
fn load_default_root_is_config_dir() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.project_root, dir);
}

#[test]
fn load_parses_sources_with_mount_and_remote() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "acme"

[local]
mount = "docs"

[shared]
mount = "vendor/shared"
remote = "git@github.com:acme/shared.git"
"#,
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.sources.len(), 2);

    let local = cfg.sources.iter().find(|s| s.name == "local").unwrap();
    assert_eq!(local.mount, dir.join("docs"));
    assert!(local.remote.is_none());
    assert_eq!(local.prefix, "local");

    let shared = cfg.sources.iter().find(|s| s.name == "shared").unwrap();
    assert_eq!(shared.mount, dir.join("vendor").join("shared"));
    assert_eq!(shared.remote.as_deref(), Some("git@github.com:acme/shared.git"));
}

#[test]
fn load_default_mount_is_project_root() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.sources[0].mount, dir);
}

#[test]
fn load_default_prefix_is_source_name() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[hello]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.sources[0].prefix, "hello");
}

#[test]
fn load_allows_explicit_empty_prefix() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\nprefix = \"\"\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.sources[0].prefix, "");
}

#[test]
fn load_rejects_multiple_empty_prefixes() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[a]
prefix = ""

[b]
prefix = ""
"#,
    )
    .unwrap();
    assert!(config::load(&dir, None).is_err());
}

#[test]
fn load_default_include_is_all_md() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.sources[0].include, vec!["**/*.md"]);
}

#[test]
fn load_expands_tilde_in_mount() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\nmount = \"~/memex-test-xyz\"\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let home = dirs::home_dir().unwrap();
    assert_eq!(cfg.sources[0].mount, home.join("memex-test-xyz"));
}

#[test]
fn load_expands_env_var_in_mount() {
    let dir = mk_tmp("memex-parse-");
    std::env::set_var("MEMEX_TEST_VAR", "/tmp/envtest");
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\nmount = \"${MEMEX_TEST_VAR}/sub\"\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.sources[0].mount, PathBuf::from("/tmp/envtest/sub"));
    std::env::remove_var("MEMEX_TEST_VAR");
}

// ---------- slug ----------

#[test]
fn blueprint_id_uses_source_prefix_plus_relpath() {
    let env = create_env();
    let source = &env.cfg.sources[0];
    let fp = source.mount.join("vision.md");
    assert_eq!(env.cfg.blueprint_id(source, &fp), "testsource/vision");
}

#[test]
fn blueprint_id_includes_subdirectories() {
    let env = create_env();
    let source = &env.cfg.sources[0];
    let fp = source.mount.join("tech").join("migrations.md");
    assert_eq!(env.cfg.blueprint_id(source, &fp), "testsource/tech/migrations");
}

#[test]
fn blueprint_id_omits_prefix_when_empty() {
    let dir = mk_tmp("memex-slug-");
    std::fs::create_dir_all(dir.join("bp")).unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\nmount = \"bp\"\nprefix = \"\"\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let source = &cfg.sources[0];
    let fp = source.mount.join("vision.md");
    assert_eq!(cfg.blueprint_id(source, &fp), "vision");
}

#[test]
fn resolve_blueprint_matches_by_prefix() {
    let env = create_env();
    let resolved = env.cfg.resolve_blueprint("testsource/vision").unwrap();
    assert_eq!(resolved.source.name, "testsource");
    assert_eq!(resolved.file_path, env.source_mount.join("vision.md"));
}

#[test]
fn resolve_blueprint_errors_for_unknown_prefix() {
    let env = create_env();
    let err = env.cfg.resolve_blueprint("nope/foo").unwrap_err().to_string();
    assert!(err.contains("No source"), "got: {err}");
}

#[test]
fn resolve_blueprint_uses_empty_prefix_fallback() {
    let dir = mk_tmp("memex-resolve-");
    std::fs::create_dir_all(dir.join("notes")).unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[notes]
mount = "notes"

[bare]
prefix = ""
"#,
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    // "notes/foo" → source with prefix "notes"
    let r1 = cfg.resolve_blueprint("notes/foo").unwrap();
    assert_eq!(r1.source.name, "notes");
    // "TODOS" → fallback to empty-prefix source
    let r2 = cfg.resolve_blueprint("TODOS").unwrap();
    assert_eq!(r2.source.name, "bare");
    assert_eq!(r2.file_path, dir.join("TODOS.md"));
}

#[test]
fn resolve_blueprint_picks_longest_prefix() {
    let dir = mk_tmp("memex-resolve-");
    std::fs::create_dir_all(dir.join("a")).unwrap();
    std::fs::create_dir_all(dir.join("a-b")).unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[a]
mount = "a"

[a-b]
mount = "a-b"
"#,
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    // "a-b/x" must resolve to `a-b`, not `a`.
    let r = cfg.resolve_blueprint("a-b/x").unwrap();
    assert_eq!(r.source.name, "a-b");
}

// ---------- enumeration + globs ----------

#[test]
fn all_blueprints_finds_md_files() {
    let env = create_env();
    std::fs::write(env.source_mount.join("alpha.md"), "# Alpha").unwrap();
    std::fs::write(env.source_mount.join("beta.md"), "# Beta").unwrap();
    let entries = env.cfg.all_blueprints();
    let names: Vec<String> = entries
        .iter()
        .map(|(_, p)| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"alpha".to_string()));
    assert!(names.contains(&"beta".to_string()));
}

#[test]
fn all_blueprints_skips_foreign_git_subtree() {
    let env = create_env();
    std::fs::write(env.source_mount.join("visible.md"), "# V").unwrap();

    // Create a foreign git repo inside the source.
    let foreign = env.source_mount.join("foreign");
    std::fs::create_dir_all(foreign.join(".git")).unwrap();
    std::fs::write(foreign.join("hidden.md"), "# H").unwrap();

    let entries = env.cfg.all_blueprints();
    let names: Vec<String> = entries
        .iter()
        .map(|(_, p)| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"visible".to_string()));
    assert!(!names.contains(&"hidden".to_string()));
}

#[test]
fn include_globs_filter_files() {
    let dir = mk_tmp("memex-glob-");
    std::fs::create_dir_all(dir.join("bp").join("sub")).unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\nmount = \"bp\"\ninclude = [\"sub/**/*.md\"]\n",
    )
    .unwrap();
    std::fs::write(dir.join("bp").join("top.md"), "# Top").unwrap();
    std::fs::write(dir.join("bp").join("sub").join("nested.md"), "# N").unwrap();

    let cfg = config::load(&dir, None).unwrap();
    let entries = cfg.all_blueprints();
    let names: Vec<String> = entries
        .iter()
        .map(|(_, p)| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"nested".to_string()));
    assert!(!names.contains(&"top".to_string()));
}

#[test]
fn exclude_globs_remove_files() {
    let dir = mk_tmp("memex-glob-");
    std::fs::create_dir_all(dir.join("bp").join("drafts")).unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        "project_name = \"p\"\n\n[s]\nmount = \"bp\"\nexclude = [\"drafts/**\"]\n",
    )
    .unwrap();
    std::fs::write(dir.join("bp").join("keep.md"), "# K").unwrap();
    std::fs::write(dir.join("bp").join("drafts").join("skip.md"), "# S").unwrap();

    let cfg = config::load(&dir, None).unwrap();
    let entries = cfg.all_blueprints();
    let names: Vec<String> = entries
        .iter()
        .map(|(_, p)| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"keep".to_string()));
    assert!(!names.contains(&"skip".to_string()));
}

// ---------- db_path, title ----------

#[test]
fn db_path_is_in_cache_dir() {
    let env = create_env();
    let base = dirs::cache_dir().unwrap();
    let expected = base
        .join("memex")
        .join("indexes")
        .join("testproject")
        .join("vector_index.sqlite");
    assert_eq!(env.cfg.db_path(), expected);
}

#[test]
fn extract_title_returns_first_h1() {
    let env = create_env();
    assert_eq!(env.cfg.extract_title("# My Title\n\nBody text."), "My Title");
}

#[test]
fn extract_title_returns_untitled_when_no_heading() {
    let env = create_env();
    assert_eq!(env.cfg.extract_title("No heading here."), "Untitled");
}

// ---------- collisions ----------

// ---------- index_filename ----------

#[test]
fn blueprint_id_uses_parent_dir_for_index_filename_match() {
    let dir = mk_tmp("memex-idx-");
    std::fs::create_dir_all(dir.join("deps").join("ecto_context")).unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[deps]
mount = "deps"
index_filename = "usage-rules.md"
"#,
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let source = &cfg.sources[0];

    // Index file → slug drops filename, uses parent dir.
    let idx_path = source.mount.join("ecto_context").join("usage-rules.md");
    assert_eq!(cfg.blueprint_id(source, &idx_path), "deps/ecto_context");

    // Non-matching file → default slug derivation.
    let other = source.mount.join("ecto_context").join("other.md");
    assert_eq!(cfg.blueprint_id(source, &other), "deps/ecto_context/other");
}

#[test]
fn blueprint_id_handles_index_file_at_mount_root() {
    let dir = mk_tmp("memex-idx-");
    std::fs::create_dir_all(dir.join("bp")).unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[bp]
mount = "bp"
index_filename = "usage-rules.md"
"#,
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let source = &cfg.sources[0];
    let root_idx = source.mount.join("usage-rules.md");
    // Slug collapses to just the prefix.
    assert_eq!(cfg.blueprint_id(source, &root_idx), "bp");
}

#[test]
fn resolve_blueprint_finds_index_file_when_default_absent() {
    let dir = mk_tmp("memex-idx-");
    std::fs::create_dir_all(dir.join("deps").join("ecto_context")).unwrap();
    std::fs::write(
        dir.join("deps").join("ecto_context").join("usage-rules.md"),
        "# ecto_context",
    )
    .unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[deps]
mount = "deps"
index_filename = "usage-rules.md"
"#,
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let r = cfg.resolve_blueprint("deps/ecto_context").unwrap();
    assert_eq!(
        r.file_path,
        dir.join("deps").join("ecto_context").join("usage-rules.md")
    );
}

#[test]
fn ensure_initialized_detects_index_filename_collision_within_source() {
    let dir = mk_tmp("memex-idx-");
    std::fs::create_dir_all(dir.join("deps").join("foo")).unwrap();
    // Both files produce slug "deps/foo" → collision.
    std::fs::write(dir.join("deps").join("foo.md"), "# foo").unwrap();
    std::fs::write(
        dir.join("deps").join("foo").join("usage-rules.md"),
        "# foo-index",
    )
    .unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[deps]
mount = "deps"
index_filename = "usage-rules.md"
"#,
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let err = cfg.ensure_initialized().unwrap_err().to_string();
    assert!(err.contains("slug collision"), "got: {err}");
}

#[test]
fn ensure_initialized_detects_slug_collisions() {
    let dir = mk_tmp("memex-coll-");
    std::fs::create_dir_all(dir.join("a")).unwrap();
    std::fs::create_dir_all(dir.join("b")).unwrap();
    std::fs::write(dir.join("a").join("x.md"), "# X").unwrap();
    std::fs::write(dir.join("b").join("x.md"), "# X").unwrap();
    std::fs::write(
        dir.join("memex.toml"),
        r#"project_name = "p"

[s1]
mount = "a"
prefix = "shared"

[s2]
mount = "b"
prefix = "shared"
"#,
    )
    .unwrap();

    let cfg = config::load(&dir, None).unwrap();
    let err = cfg.ensure_initialized().unwrap_err().to_string();
    assert!(err.contains("slug collision"), "got: {err}");
}
