# Debugging Interactive Terminal Commands

When debugging TUI commands like `wt beta select`, use MCP's `node-terminal` tools to test interactively.

## Debugging Workflow

### 1. Create Test Environment

```bash
cargo run --bin setup-select-test
```

This creates a reproducible test repo at `/tmp/wt-select-test/test-repo`.

### 2. Test in MCP Terminal

```typescript
// Create terminal and navigate to test repo
mcp__node-terminal__terminal_create({ sessionId: "test" })
mcp__node-terminal__terminal_write({ sessionId: "test", input: "cd /tmp/wt-select-test/test-repo" })
mcp__node-terminal__terminal_send_key({ sessionId: "test", key: "enter" })

// Run with debug logging
mcp__node-terminal__terminal_write({
  sessionId: "test",
  input: "RUST_LOG=worktrunk=debug cargo run --quiet -- beta select 2> test.log"
})
mcp__node-terminal__terminal_send_key({ sessionId: "test", key: "enter" })

// Test the interaction
mcp__node-terminal__terminal_write({ sessionId: "test", input: "3" })
mcp__node-terminal__terminal_read({ sessionId: "test" })
```

### 3. If Synthetic Test Works But User Reports Issues

**Test on the actual repository!** Environment-specific issues (git config, shell config) won't appear in isolated test environments.

```typescript
// Use -C to test in user's actual repo
mcp__node-terminal__terminal_write({
  sessionId: "test",
  input: "RUST_LOG=worktrunk=debug cargo run --quiet -- -C /path/to/actual/repo beta select 2> debug.log"
})
```

### 4. Analyze Logs

```bash
tail -100 debug.log | grep -E "error|hang|stuck"
```

## Important Flags

- **`-C <path>`**: Set working directory (alternative to `cd`)
- **`--source`**: Use local source (only needed with installed `wt`, not with `cargo run`)

```bash
# Testing with cargo run (already uses local source):
cargo run --quiet -- -C /path/to/repo beta select

# Testing with installed wt:
wt --source -C /path/to/repo beta select
```

## MCP Limitations

MCP terminals use pseudo-TTY, not real terminals. If tests pass in MCP but users report issues, the bug is likely environment-specific. Always test on the actual problematic repository.
