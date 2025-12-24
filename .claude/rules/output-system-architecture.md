# Output System Architecture

## Shell Integration

Worktrunk uses file-based directive passing for shell integration:

1. Shell wrapper creates a temp file via `mktemp`
2. Shell wrapper sets `WORKTRUNK_DIRECTIVE_FILE` env var to the file path
3. wt writes shell commands (like `cd '/path'`) to that file
4. Shell wrapper sources the file after wt exits

When `WORKTRUNK_DIRECTIVE_FILE` is not set (direct binary call), commands execute
directly and shell integration hints are shown.

## Output Functions

The output system handles shell integration automatically. Just call output
functions — they do the right thing regardless of whether shell integration is
active.

```rust
// NEVER DO THIS - don't check mode in command code
if is_shell_integration_active() {
    // different behavior
}

// ALWAYS DO THIS - just call output functions
output::print(success_message("Created worktree"))?;
output::change_directory(&path)?;  // Writes to directive file if set, else no-op
```

**Output functions** (`src/output/global.rs`):

| Function | Destination | Purpose |
|----------|-------------|---------|
| `print(message)` | stderr | Status messages (use with formatting functions) |
| `shell_integration_hint(message)` | stderr | Hints suppressed when shell integration active |
| `gutter(content)` | stderr | Gutter-formatted quoted content |
| `blank()` | stderr | Visual separation |
| `table(content)` | stdout | Primary output (pipeable) |
| `data(content)` | stdout | Structured data (JSON) |
| `change_directory(path)` | directive file | Shell cd after wt exits |
| `execute(command)` | directive file | Shell command after wt exits |
| `flush()` | both | Flush buffers |
| `flush_for_stderr_prompt()` | both | Flush before interactive prompts |
| `terminate_output()` | stderr | Reset ANSI state on stderr |
| `is_shell_integration_active()` | — | Check if directive file set (rarely needed) |

**Message formatting functions** (`worktrunk::styling`):

| Function | Symbol | Color |
|----------|--------|-------|
| `success_message()` | ✓ | green |
| `progress_message()` | ◎ | cyan |
| `info_message()` | ○ | — |
| `warning_message()` | ▲ | yellow |
| `hint_message()` | ↳ | dim |
| `error_message()` | ✗ | red |

## stdout vs stderr

- **stdout**: Primary output (tables, JSON) — pipeable
- **stderr**: Status messages (progress, success, errors, hints)
- **directive file**: Shell commands (cd, exec)

This separation makes `wt list | grep feature` work.

## Security

`WORKTRUNK_DIRECTIVE_FILE` is automatically removed from spawned subprocesses
(via `shell_exec::run()`). This prevents hooks from writing to the directive
file.

## Adding New Output Functions

Add the function to `global.rs`. The pattern:
- **Primary output** (data the command produces) → stdout via `table()` or `data()`
- **Status messages** (progress, success, errors) → stderr via `print()`
- **Directives** (cd, exec) → directive file via `change_directory()`, `execute()`
