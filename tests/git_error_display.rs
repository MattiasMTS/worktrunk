use insta::assert_snapshot;
use worktrunk::git::GitError;

#[test]
fn display_worktree_removal_failed() {
    let err = GitError::WorktreeRemovalFailed {
        branch: "feature-x".to_string(),
        path: std::path::PathBuf::from("/tmp/repo.feature-x"),
        error: "fatal: worktree is dirty\nerror: could not remove worktree".to_string(),
    };

    assert_snapshot!("worktree_removal_failed", err.to_string());
}
