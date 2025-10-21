use crate::common::{TestRepo, make_snapshot_cmd, setup_snapshot_settings};
use insta_cmd::assert_cmd_snapshot;
use std::fs;

/// Helper to create snapshot with normalized paths and SHAs
fn snapshot_switch(test_name: &str, repo: &TestRepo, args: &[&str]) {
    let settings = setup_snapshot_settings(repo);
    settings.bind(|| {
        let mut cmd = make_snapshot_cmd(repo, "switch", args, None);
        assert_cmd_snapshot!(test_name, cmd);
    });
}

#[test]
fn test_post_start_commands_no_config() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Switch without project config should work normally
    snapshot_switch("post_start_no_config", &repo, &["--create", "feature"]);
}

#[test]
fn test_post_start_commands_empty_array() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create empty project config
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(config_dir.join("wt.toml"), "post-start-commands = []\n")
        .expect("Failed to write config");

    repo.commit("Add empty config");

    // Should work without prompting
    snapshot_switch("post_start_empty_array", &repo, &["--create", "feature"]);
}

#[test]
fn test_post_start_commands_with_approval() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with a simple command
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-start-commands = ["echo 'Setup complete'"]"#,
    )
    .expect("Failed to write config");

    repo.commit("Add config");

    // Pre-approve the command by setting up the user config
    // This simulates the command being already approved
    let home_dir = std::env::var("HOME").unwrap();
    let user_config_dir =
        std::path::Path::new(&home_dir).join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).ok();
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'Setup complete'"
"#,
    )
    .ok();

    // Command should execute without prompting
    snapshot_switch("post_start_with_approval", &repo, &["--create", "feature"]);
}

#[test]
fn test_post_start_commands_invalid_toml() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create invalid TOML
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        "post-start-commands = [invalid syntax\n",
    )
    .expect("Failed to write config");

    repo.commit("Add invalid config");

    // Should continue without executing commands, showing warning
    snapshot_switch("post_start_invalid_toml", &repo, &["--create", "feature"]);
}

#[test]
fn test_post_start_commands_failing_command() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with a command that will fail
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-start-commands = ["exit 1"]"#,
    )
    .expect("Failed to write config");

    repo.commit("Add config with failing command");

    // Pre-approve the command
    let home_dir = std::env::var("HOME").unwrap();
    let config_dir = std::path::Path::new(&home_dir).join("Library/Application Support/worktrunk");
    fs::create_dir_all(&config_dir).ok();
    fs::write(
        config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "exit 1"
"#,
    )
    .ok();

    // Should show warning but continue (worktree should still be created)
    snapshot_switch(
        "post_start_failing_command",
        &repo,
        &["--create", "feature"],
    );
}

#[test]
fn test_post_start_commands_multiple_commands() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with multiple commands
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-start-commands = ["echo 'First'", "echo 'Second'"]"#,
    )
    .expect("Failed to write config");

    repo.commit("Add config with multiple commands");

    // Pre-approve both commands
    let home_dir = std::env::var("HOME").unwrap();
    let config_dir = std::path::Path::new(&home_dir).join("Library/Application Support/worktrunk");
    fs::create_dir_all(&config_dir).ok();
    fs::write(
        config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'First'"

[[approved-commands]]
project = "main"
command = "echo 'Second'"
"#,
    )
    .ok();

    // Both commands should execute
    snapshot_switch(
        "post_start_multiple_commands",
        &repo,
        &["--create", "feature"],
    );
}

#[test]
fn test_post_start_commands_template_expansion() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create project config with template variables
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).expect("Failed to create .config dir");
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-start-commands = [
    "echo 'Repo: {repo}' > info.txt",
    "echo 'Branch: {branch}' >> info.txt",
    "echo 'Worktree: {worktree}' >> info.txt",
    "echo 'Root: {repo_root}' >> info.txt"
]"#,
    )
    .expect("Failed to write config");

    repo.commit("Add config with templates");

    // Pre-approve all commands
    let home_dir = std::env::var("HOME").unwrap();
    let user_config_dir =
        std::path::Path::new(&home_dir).join("Library/Application Support/worktrunk");
    fs::create_dir_all(&user_config_dir).ok();
    let repo_name = "main";
    fs::write(
        user_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "main"
command = "echo 'Repo: {repo}' > info.txt"

[[approved-commands]]
project = "main"
command = "echo 'Branch: {branch}' >> info.txt"

[[approved-commands]]
project = "main"
command = "echo 'Worktree: {worktree}' >> info.txt"

[[approved-commands]]
project = "main"
command = "echo 'Root: {repo_root}' >> info.txt"
"#,
    )
    .ok();

    // Commands should execute with expanded templates
    snapshot_switch(
        "post_start_template_expansion",
        &repo,
        &["--create", "feature/test"],
    );

    // Verify template expansion actually worked by checking the output file
    // The worktree path should be ../main.feature-test (slashes replaced with dashes)
    let worktree_path = repo
        .root_path()
        .parent()
        .unwrap()
        .join(format!("{}.feature-test", repo_name));
    let info_file = worktree_path.join("info.txt");

    assert!(
        info_file.exists(),
        "info.txt should have been created in the worktree"
    );

    let contents = fs::read_to_string(&info_file).expect("Failed to read info.txt");

    // Verify that template variables were actually expanded, not left as literals
    assert!(
        contents.contains(&format!("Repo: {}", repo_name)),
        "Should contain expanded repo name, got: {}",
        contents
    );
    assert!(
        contents.contains("Branch: feature-test"),
        "Should contain expanded branch name (sanitized), got: {}",
        contents
    );
    assert!(
        contents.contains(&format!(
            "Worktree: {}",
            worktree_path.canonicalize().unwrap().display()
        )),
        "Should contain expanded worktree path, got: {}",
        contents
    );
    assert!(
        contents.contains(&format!(
            "Root: {}",
            repo.root_path().canonicalize().unwrap().display()
        )),
        "Should contain expanded repo root path, got: {}",
        contents
    );

    // Make sure they're NOT the literal template strings
    assert!(
        !contents.contains("{repo}"),
        "Should not contain literal {{repo}} placeholder"
    );
    assert!(
        !contents.contains("{branch}"),
        "Should not contain literal {{branch}} placeholder"
    );
    assert!(
        !contents.contains("{worktree}"),
        "Should not contain literal {{worktree}} placeholder"
    );
    assert!(
        !contents.contains("{repo_root}"),
        "Should not contain literal {{repo_root}} placeholder"
    );
}
