mod common;

use std::path::PathBuf;

use memex::config;

use common::{create_env, mk_tmp};

#[test]
fn discover_finds_memex_toml_in_given_directory() {
    let dir = mk_tmp("memex-discover-");
    std::fs::write(
        dir.join("memex.toml"),
        "[s]\npath = \"/tmp/s\"\nfolders = [\"f\"]\n",
    )
    .unwrap();
    let p = config::discover(&dir).unwrap();
    assert_eq!(p, dir.join("memex.toml"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn discover_finds_config_in_dot_memex() {
    let dir = mk_tmp("memex-discover-");
    std::fs::create_dir_all(dir.join(".memex")).unwrap();
    std::fs::write(
        dir.join(".memex").join("memex.toml"),
        "[s]\npath = \"/tmp/s\"\nfolders = [\"f\"]\n",
    )
    .unwrap();
    let p = config::discover(&dir).unwrap();
    assert_eq!(p, dir.join(".memex").join("memex.toml"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn discover_finds_config_in_config_subdir() {
    let dir = mk_tmp("memex-discover-");
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::write(
        dir.join("config").join("memex.toml"),
        "[s]\npath = \"/tmp/s\"\nfolders = [\"f\"]\n",
    )
    .unwrap();
    let p = config::discover(&dir).unwrap();
    assert_eq!(p, dir.join("config").join("memex.toml"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn discover_errors_when_no_config_found() {
    let dir = mk_tmp("memex-discover-");
    let err = config::discover(&dir).unwrap_err().to_string();
    assert!(err.contains("No memex.toml found"), "got: {err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_sources_from_toml() {
    let dir = mk_tmp("memex-parse-");
    let cfg_path = dir.join("memex.toml");
    std::fs::write(
        &cfg_path,
        r#"
[local]
path = "/tmp/local-blueprints"
folders = ["docs", "arch"]

[shared]
path = "/tmp/shared"
remote = "git@github.com:acme/shared.git"
folders = ["runbooks"]
"#,
    )
    .unwrap();

    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.project_dir, dir);
    assert_eq!(cfg.sources.len(), 2);

    let local = cfg.sources.iter().find(|s| s.name == "local").unwrap();
    assert_eq!(local.path, PathBuf::from("/tmp/local-blueprints"));
    assert!(local.remote.is_none());
    assert_eq!(local.folders, vec!["docs", "arch"]);

    let shared = cfg.sources.iter().find(|s| s.name == "shared").unwrap();
    assert_eq!(shared.remote.as_deref(), Some("git@github.com:acme/shared.git"));
    assert_eq!(shared.folders, vec!["runbooks"]);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_expands_tilde_in_paths() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "[s]\npath = \"~/.memex/test\"\nfolders = [\"f\"]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let home = dirs::home_dir().unwrap();
    assert_eq!(cfg.sources[0].path, home.join(".memex").join("test"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_resolves_project_dir_when_config_in_config_subdir() {
    let dir = mk_tmp("memex-parse-");
    let cfg_dir = dir.join("config");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("memex.toml"),
        "[s]\npath = \"/tmp/s\"\nfolders = [\"f\"]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.project_dir, dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_resolves_project_dir_when_config_in_dot_memex() {
    let dir = mk_tmp("memex-parse-");
    let cfg_dir = dir.join(".memex");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("memex.toml"),
        "[s]\npath = \"/tmp/s\"\nfolders = [\"f\"]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.project_dir, dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn path_defaults_to_dot_memex_name_when_omitted() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "[mysource]\nfolders = [\"docs\"]\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(
        cfg.sources[0].path,
        dir.join(".memex").join("mysource")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn folders_defaults_to_dot_when_omitted() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "[s]\npath = \"/tmp/s\"\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    assert_eq!(cfg.sources[0].folders, vec!["."]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parses_project_true_flag() {
    let dir = mk_tmp("memex-parse-");
    std::fs::write(
        dir.join("memex.toml"),
        "[proj]\npath = \"/tmp/p\"\nproject = true\n\n[shared]\npath = \"/tmp/s\"\n",
    )
    .unwrap();
    let cfg = config::load(&dir, None).unwrap();
    let proj = cfg.sources.iter().find(|s| s.name == "proj").unwrap();
    let shared = cfg.sources.iter().find(|s| s.name == "shared").unwrap();
    assert!(proj.project);
    assert!(!shared.project);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn db_path_is_under_project_dir_dot_memex() {
    let env = create_env();
    assert_eq!(
        env.cfg.db_path(),
        env.project_dir.join(".memex").join("vector_index.sqlite")
    );
}

#[test]
fn blueprint_id_returns_source_namespaced_id() {
    let env = create_env();
    let source = &env.cfg.sources[0];
    let fp = source.path.join("test").join("tech").join("my-blueprint.md");
    assert_eq!(
        env.cfg.blueprint_id(source, &fp),
        "testsource/test/tech/my-blueprint"
    );
}

#[test]
fn blueprint_folder_returns_source_namespaced_folder() {
    let env = create_env();
    let source = &env.cfg.sources[0];
    let fp = source.path.join("test").join("tech").join("my-blueprint.md");
    assert_eq!(
        env.cfg.blueprint_folder(source, &fp),
        "testsource/test/tech"
    );
}

#[test]
fn resolve_blueprint_resolves_id_to_source_and_path() {
    let env = create_env();
    let resolved = env.cfg.resolve_blueprint("testsource/test/tech/my-bp").unwrap();
    assert_eq!(resolved.source.name, "testsource");
    assert_eq!(
        resolved.file_path,
        env.cfg.sources[0]
            .path
            .join("test")
            .join("tech")
            .join("my-bp.md")
    );
    assert_eq!(resolved.folder, "testsource/test/tech");
}

#[test]
fn resolve_blueprint_errors_for_unknown_source() {
    let env = create_env();
    let err = env.cfg.resolve_blueprint("unknown/test").unwrap_err().to_string();
    assert!(err.contains("No source found"), "got: {err}");
}

#[test]
fn all_blueprints_finds_md_files_across_sources() {
    let env = create_env();
    let base = env.cfg.sources[0].path.join("test").join("tech");
    std::fs::write(base.join("alpha.md"), "# Alpha").unwrap();
    std::fs::write(base.join("beta.md"), "# Beta").unwrap();
    let entries = env.cfg.all_blueprints();
    let names: Vec<String> = entries
        .iter()
        .map(|(_, p)| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"alpha".to_string()));
    assert!(names.contains(&"beta".to_string()));
}

#[test]
fn all_folders_returns_namespaced_folders() {
    let env = create_env();
    assert_eq!(env.cfg.all_folders(), vec!["testsource/test/tech"]);
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

#[test]
fn ensure_initialized_creates_dot_memex_and_source_repo() {
    let tmp = mk_tmp("memex-init-");
    let project_dir = tmp.join("project");
    std::fs::create_dir_all(&project_dir).unwrap();
    let source_dir = tmp.join("newsource");

    std::fs::write(
        project_dir.join("memex.toml"),
        format!(
            "[newsource]\npath = \"{}\"\nfolders = [\"docs\"]\n",
            source_dir.display()
        ),
    )
    .unwrap();

    let cfg = config::load(&project_dir, None).unwrap();
    cfg.ensure_initialized().unwrap();

    assert!(project_dir.join(".memex").exists());
    assert!(source_dir.join(".git").exists());
    assert!(source_dir.join("docs").exists());

    // idempotent
    cfg.ensure_initialized().unwrap();

    let _ = std::fs::remove_dir_all(&tmp);
}
