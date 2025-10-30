pub mod command_approval;
mod command_executor;
pub mod completion;
pub mod config;
pub mod configure_shell;
pub mod dev;
pub mod init;
pub mod list;
pub mod merge;
pub mod process;
pub mod worktree;

pub use completion::{handle_complete, handle_completion};
pub use config::{handle_config_help, handle_config_init, handle_config_list};
pub use configure_shell::{ConfigAction, handle_configure_shell};
pub use dev::handle_dev_run_hook;
pub use init::handle_init;
pub use list::handle_list;
pub use merge::handle_merge;
pub use worktree::{handle_push, handle_remove, handle_switch};

// Re-export Shell from the canonical location
pub use worktrunk::shell::Shell;

/// Format command execution label with optional command name.
///
/// Examples:
/// - `format_command_label("post-create", Some("install"))` → `"Running post-create: install"`
/// - `format_command_label("post-create", None)` → `"Running post-create:"`
pub fn format_command_label(command_type: &str, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("Running {command_type}: {name}"),
        None => format!("Running {command_type}:"),
    }
}
