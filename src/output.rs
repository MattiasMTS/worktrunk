//! Output and presentation layer for worktree commands.
//!
//! This module handles all presentation logic, keeping it separate from business logic.
//! It provides:
//! - Directive protocol types for shell integration
//! - Message formatting for results
//! - Output rendering for both internal and non-internal modes

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use worktrunk::git::GitError;
use worktrunk::shell::Shell;
use worktrunk::styling::{AnstyleStyle, println};

use crate::commands::worktree::{RemoveResult, SwitchResult};

/// A directive for the shell wrapper to execute
#[derive(Debug, Clone)]
pub enum Directive {
    /// Change directory to the given path
    ChangeDirectory(PathBuf),
    /// Execute a command in the current directory
    Execute(String),
    /// Print a message to the user
    Message(String),
}

/// Output containing multiple directives for shell integration
#[derive(Default)]
pub struct DirectiveOutput {
    directives: Vec<Directive>,
}

impl DirectiveOutput {
    /// Create a new directive output
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a directive
    pub fn add(&mut self, directive: Directive) -> &mut Self {
        self.directives.push(directive);
        self
    }

    /// Write directives to stdout with NUL termination
    ///
    /// Format: `<directive_content>\0<directive_content>\0...`
    /// Each directive is NUL-terminated to support multi-line commands.
    pub fn write_to_stdout(&self) -> io::Result<()> {
        let mut stdout = io::stdout();

        for directive in &self.directives {
            match directive {
                Directive::ChangeDirectory(path) => {
                    write!(stdout, "__WORKTRUNK_CD__{}\0", path.display())?;
                }
                Directive::Execute(cmd) => {
                    write!(stdout, "__WORKTRUNK_EXEC__{}\0", cmd)?;
                }
                Directive::Message(msg) => {
                    write!(stdout, "{}\0", msg)?;
                }
            }
        }

        // Final newline for readability in logs
        stdout.write_all(b"\n")?;
        Ok(())
    }
}

/// Format a message for a switch operation
pub fn format_switch_message(result: &SwitchResult, branch: &str) -> String {
    use anstyle::{AnsiColor, Color};
    let green = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let green_bold = green.bold();

    match result {
        SwitchResult::ExistingWorktree(_) => {
            format!(
                "✅ {green}Switched to worktree for {green_bold}{branch}{green_bold:#}{green:#}"
            )
        }
        SwitchResult::CreatedWorktree {
            path,
            created_branch,
        } => {
            let dim = AnstyleStyle::new().dimmed();
            if *created_branch {
                format!(
                    "✅ {green}Created new worktree for {green_bold}{branch}{green_bold:#}{green:#}\n  {dim}Path: {}{dim:#}",
                    path.display()
                )
            } else {
                format!(
                    "✅ {green}Added worktree for {green_bold}{branch}{green_bold:#}{green:#}\n  {dim}Path: {}{dim:#}",
                    path.display()
                )
            }
        }
    }
}

/// Handle output for a switch operation
///
/// In internal mode: outputs directives for shell wrapper
/// In non-internal mode: prints message and executes command
pub fn handle_switch_output(
    result: &SwitchResult,
    branch: &str,
    execute: Option<&str>,
    internal: bool,
) -> Result<(), GitError> {
    if internal {
        // Internal mode: output directives for shell wrapper
        let mut output = DirectiveOutput::new();
        output.add(Directive::ChangeDirectory(result.path().clone()));
        output.add(Directive::Message(format_switch_message(result, branch)));

        if let Some(cmd) = execute {
            output.add(Directive::Execute(cmd.to_string()));
        }

        output
            .write_to_stdout()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;
    } else {
        // Non-internal mode: print message and execute command
        println!("{}", format_switch_message(result, branch));

        if let Some(cmd) = execute {
            // Execute command after showing message
            println!();
            execute_command_in_worktree(result.path(), cmd)?;
        } else {
            // Show shell integration hint if no command to execute
            println!();
            println!("{}", shell_integration_hint());
        }
    }

    Ok(())
}

/// Format a message for a remove operation
pub fn format_remove_message(result: &RemoveResult) -> String {
    use anstyle::{AnsiColor, Color};
    let green = AnstyleStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let green_bold = green.bold();

    match result {
        RemoveResult::AlreadyOnDefault(branch) => {
            format!(
                "✅ {green}Already on default branch {green_bold}{branch}{green_bold:#}{green:#}"
            )
        }
        RemoveResult::RemovedWorktree { primary_path } => {
            let dim = AnstyleStyle::new().dimmed();
            format!(
                "✅ {green}Removed worktree and returned to primary{green:#}\n  {dim}Path: {}{dim:#}",
                primary_path.display()
            )
        }
        RemoveResult::SwitchedToDefault(branch) => {
            format!(
                "✅ {green}Switched to default branch {green_bold}{branch}{green_bold:#}{green:#}"
            )
        }
    }
}

/// Handle output for a remove operation
///
/// In internal mode: outputs directives for shell wrapper (only for RemovedWorktree)
/// In non-internal mode: prints message
pub fn handle_remove_output(result: &RemoveResult, internal: bool) -> Result<(), GitError> {
    if internal {
        // Internal mode: only RemovedWorktree needs a CD directive
        if let RemoveResult::RemovedWorktree { primary_path } = result {
            let mut output = DirectiveOutput::new();
            output.add(Directive::ChangeDirectory(primary_path.clone()));
            output.add(Directive::Message(format_remove_message(result)));

            output
                .write_to_stdout()
                .map_err(|e| GitError::CommandFailed(e.to_string()))?;
        }
        // Other cases don't need directives in internal mode
    } else {
        // Non-internal mode: print message and hint
        println!("{}", format_remove_message(result));

        // Show shell integration hint for RemovedWorktree
        if matches!(result, RemoveResult::RemovedWorktree { .. }) {
            println!();
            println!("{}", shell_integration_hint());
        }
    }

    Ok(())
}

/// Generate hint message for shell integration setup
pub fn shell_integration_hint() -> String {
    if let Some(config_path) = Shell::is_integration_configured() {
        format!(
            "Shell integration configured. Restart your shell or run: source {}",
            config_path.display()
        )
    } else {
        "To enable automatic cd, run: wt configure-shell".to_string()
    }
}

/// Execute a command in the given worktree directory
pub fn execute_command_in_worktree(worktree_path: &Path, command: &str) -> Result<(), GitError> {
    #[cfg(target_os = "windows")]
    let (shell, shell_arg) = ("cmd", "/C");
    #[cfg(not(target_os = "windows"))]
    let (shell, shell_arg) = ("sh", "-c");

    let status = std::process::Command::new(shell)
        .arg(shell_arg)
        .arg(command)
        .current_dir(worktree_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| GitError::CommandFailed(format!("Failed to execute command: {}", e)))?;

    if !status.success() {
        return Err(GitError::CommandFailed(format!(
            "Command '{}' failed with exit code {}",
            command,
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}
