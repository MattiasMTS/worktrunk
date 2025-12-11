//! Template expansion utilities for worktrunk
//!
//! Uses minijinja for template rendering. Single generic function with escaping flag:
//! - `shell_escape: true` — Shell-escaped for safe command execution
//! - `shell_escape: false` — Literal values for filesystem paths
//!
//! All templates support Jinja2 syntax including filters, conditionals, and loops.

use minijinja::Environment;
use std::collections::HashMap;

/// Sanitize a branch name for use in filesystem paths.
///
/// Replaces path separators (`/` and `\`) with dashes to prevent directory traversal
/// and ensure the branch name is a single path component.
///
/// # Examples
/// ```
/// use worktrunk::config::sanitize_branch_name;
///
/// assert_eq!(sanitize_branch_name("feature/foo"), "feature-foo");
/// assert_eq!(sanitize_branch_name("user\\task"), "user-task");
/// assert_eq!(sanitize_branch_name("simple-branch"), "simple-branch");
/// ```
pub fn sanitize_branch_name(branch: &str) -> String {
    branch.replace(['/', '\\'], "-")
}

/// Expand a template with variable substitution.
///
/// # Arguments
/// * `template` - Template string using Jinja2 syntax (e.g., `{{ branch }}`)
/// * `vars` - Variables to substitute. Callers should sanitize branch names with
///   [`sanitize_branch_name`] before inserting.
/// * `shell_escape` - If true, shell-escape all values for safe command execution.
///   If false, substitute values literally (for filesystem paths).
///
/// # Examples
/// ```
/// use worktrunk::config::{expand_template, sanitize_branch_name};
/// use std::collections::HashMap;
///
/// // For shell commands (escaped)
/// let branch = sanitize_branch_name("feature/foo");
/// let mut vars = HashMap::new();
/// vars.insert("branch", branch.as_str());
/// vars.insert("repo", "myrepo");
/// let cmd = expand_template("echo {{ branch }} in {{ repo }}", &vars, true).unwrap();
/// assert_eq!(cmd, "echo feature-foo in myrepo");
///
/// // For filesystem paths (literal)
/// let branch = sanitize_branch_name("feature/foo");
/// let mut vars = HashMap::new();
/// vars.insert("branch", branch.as_str());
/// vars.insert("main_worktree", "myrepo");
/// let path = expand_template("{{ main_worktree }}.{{ branch }}", &vars, false).unwrap();
/// assert_eq!(path, "myrepo.feature-foo");
/// ```
pub fn expand_template(
    template: &str,
    vars: &HashMap<&str, &str>,
    shell_escape: bool,
) -> Result<String, String> {
    use shell_escape::escape;
    use std::borrow::Cow;

    // Build context map, optionally shell-escaping values
    let mut context = HashMap::new();
    for (key, value) in vars {
        let val = if shell_escape {
            escape(Cow::Borrowed(*value)).to_string()
        } else {
            (*value).to_string()
        };
        context.insert(key.to_string(), minijinja::Value::from(val));
    }

    // Render template with minijinja
    let mut env = Environment::new();
    if shell_escape {
        // Preserve trailing newlines in templates (important for multiline shell commands)
        env.set_keep_trailing_newline(true);
    }
    let tmpl = env
        .template_from_str(template)
        .map_err(|e| format!("Template syntax error: {}", e))?;

    tmpl.render(minijinja::Value::from_object(context))
        .map_err(|e| format!("Template render error: {}", e))
}
