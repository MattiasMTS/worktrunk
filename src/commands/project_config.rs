use std::path::Path;
use worktrunk::config::ProjectConfig;
use worktrunk::git::{GitError, GitResultExt, Repository};

fn load_project_config_at(repo_root: &Path) -> Result<Option<ProjectConfig>, GitError> {
    ProjectConfig::load(repo_root).git_context("Failed to load project config")
}

/// Load the project configuration if it exists.
pub fn load_project_config(repo: &Repository) -> Result<Option<ProjectConfig>, GitError> {
    let repo_root = repo.worktree_root()?;
    load_project_config_at(&repo_root)
}

/// Load the project configuration, emitting a helpful hint if missing.
pub fn require_project_config(repo: &Repository) -> Result<ProjectConfig, GitError> {
    let repo_root = repo.worktree_root()?;
    let config_path = repo_root.join(".config").join("wt.toml");

    match load_project_config_at(&repo_root)? {
        Some(cfg) => Ok(cfg),
        None => {
            use worktrunk::styling::{ERROR, ERROR_EMOJI, HINT, HINT_BOLD, HINT_EMOJI, eprintln};

            eprintln!("{ERROR_EMOJI} {ERROR}No project configuration found{ERROR:#}",);
            eprintln!(
                "{HINT_EMOJI} {HINT}Create a config file at: {HINT_BOLD}{}{HINT_BOLD:#}{HINT:#}",
                config_path.display()
            );
            Err(GitError::CommandFailed(
                "No project configuration found".to_string(),
            ))
        }
    }
}
