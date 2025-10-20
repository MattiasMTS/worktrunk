use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorktrunkConfig {
    #[serde(rename = "worktree-path")]
    pub worktree_path: String,
}

impl Default for WorktrunkConfig {
    fn default() -> Self {
        Self {
            worktree_path: "{repo}.{branch}".to_string(),
        }
    }
}

fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "worktrunk").map(|dirs| dirs.config_dir().join("config.toml"))
}

pub fn load_config() -> Result<WorktrunkConfig, ConfigError> {
    let defaults = WorktrunkConfig::default();

    let mut builder = Config::builder().set_default("worktree-path", defaults.worktree_path)?;

    // Add config file if it exists
    if let Some(config_path) = get_config_path() {
        if config_path.exists() {
            builder = builder.add_source(File::from(config_path));
        }
    }

    // Add environment variables with WORKTRUNK prefix
    builder = builder.add_source(config::Environment::with_prefix("WORKTRUNK").separator("_"));

    builder.build()?.try_deserialize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorktrunkConfig::default();
        assert_eq!(config.worktree_path, "{repo}.{branch}");
    }

    #[test]
    fn test_config_serialization() {
        let config = WorktrunkConfig::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("worktree-path"));
        assert!(toml.contains("{repo}.{branch}"));
    }

    #[test]
    fn test_load_config_defaults() {
        // Without a config file or env vars, should return defaults
        let config = load_config().unwrap();
        assert_eq!(config.worktree_path, "{repo}.{branch}");
    }
}
