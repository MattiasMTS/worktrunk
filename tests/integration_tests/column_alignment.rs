use crate::common::TestRepo;
use insta::Settings;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

/// Helper for testing with specific terminal width
fn snapshot_list_with_width(test_name: &str, repo: &TestRepo, width: usize) {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    // Normalize paths
    settings.add_filter(repo.root_path().to_str().unwrap(), "[REPO]");
    for (name, path) in &repo.worktrees {
        settings.add_filter(
            path.to_str().unwrap(),
            format!("[WORKTREE_{}]", name.to_uppercase().replace('-', "_")),
        );
    }

    // Normalize git SHAs
    settings.add_filter(r"\b[0-9a-f]{7,40}\b", "[SHA]   ");
    settings.add_filter(r"\\", "/");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("wt"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("list")
            .current_dir(repo.root_path())
            .env("COLUMNS", width.to_string());
        assert_cmd_snapshot!(test_name, cmd);
    });
}

#[test]
fn test_column_alignment_varying_diff_widths() {
    let mut repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create worktrees with varying diff sizes to test alignment
    repo.add_worktree("feature-small", "feature-small");
    repo.add_worktree("feature-medium", "feature-medium");
    repo.add_worktree("feature-large", "feature-large");

    // Add files to create diffs with different digit counts
    let small_path = repo.worktrees.get("feature-small").unwrap();
    for i in 0..5 {
        std::fs::write(small_path.join(format!("file{}.txt", i)), "content").unwrap();
    }

    let medium_path = repo.worktrees.get("feature-medium").unwrap();
    for i in 0..50 {
        std::fs::write(medium_path.join(format!("file{}.txt", i)), "content").unwrap();
    }

    let large_path = repo.worktrees.get("feature-large").unwrap();
    for i in 0..500 {
        std::fs::write(large_path.join(format!("file{}.txt", i)), "content").unwrap();
    }

    // Test at a width where WT +/- column is visible
    snapshot_list_with_width("alignment_varying_diffs", &repo, 180);
}

#[test]
fn test_column_alignment_with_empty_diffs() {
    let mut repo = TestRepo::new();
    repo.commit("Initial commit");

    // Mix of worktrees with and without diffs
    repo.add_worktree("no-changes", "no-changes");

    repo.add_worktree("with-changes", "with-changes");
    let changes_path = repo.worktrees.get("with-changes").unwrap();
    std::fs::write(changes_path.join("file.txt"), "content").unwrap();

    repo.add_worktree("also-no-changes", "also-no-changes");

    // Path column should align even when some rows have diffs and others don't
    snapshot_list_with_width("alignment_empty_diffs", &repo, 180);
}

#[test]
fn test_column_alignment_extreme_diff_sizes() {
    let mut repo = TestRepo::new();
    repo.commit("Initial commit");

    // Create worktrees with extreme diff size differences
    repo.add_worktree("tiny", "tiny");
    repo.add_worktree("huge", "huge");

    let tiny_path = repo.worktrees.get("tiny").unwrap();
    std::fs::write(tiny_path.join("file.txt"), "x").unwrap();

    let huge_path = repo.worktrees.get("huge").unwrap();
    for i in 0..9999 {
        std::fs::write(huge_path.join(format!("file{}.txt", i)), "content").unwrap();
    }

    snapshot_list_with_width("alignment_extreme_diffs", &repo, 180);
}
