+++
title = "Claude Code Integration"
weight = 23

[extra]
group = "Reference"
+++

Worktrunk includes a Claude Code plugin for tracking agent status across worktrees.

## Status tracking

The plugin adds status indicators to `wt list`:

<!-- ⚠️ AUTO-GENERATED-HTML from tests/snapshots/integration__integration_tests__list__with_user_marker.snap — edit source to update -->

{% terminal() %}
<span class="prompt">$</span> wt list
  <b>Branch</b>       <b>Status</b>         <b>HEAD±</b>    <b>main↕</b>  <b>Path</b>                <b>Remote⇅</b>  <b>Commit</b>    <b>Age</b>   <b>Message</b>
@ <b>main</b>             <span class=d>^</span>                          <b>./repo</b>                       <span class=d>b834638e</span>  <span class=d>1d</span>    <span class=d>Initial commit</span>
+ feature-api      <span class=d>↑</span>  🤖              <span class=g>↑1</span>      ./repo.feature-api           <span class=d>9606cd0f</span>  <span class=d>1d</span>    <span class=d>Add REST API endpoints</span>
+ review-ui      <span class=c>?</span> <span class=d>↑</span>  💬              <span class=g>↑1</span>      ./repo.review-ui             <span class=d>afd3b353</span>  <span class=d>1d</span>    <span class=d>Add dashboard component</span>
+ <span class=d>wip-docs</span>       <span class=c>?</span><span class=d>_</span>                           <span class=d>./repo.wip-docs</span>              <span class=d>b834638e</span>  <span class=d>1d</span>    <span class=d>Initial commit</span>

⚪ <span class=d>Showing 4 worktrees, 2 ahead</span>
{% end %}

<!-- END AUTO-GENERATED -->

- `🤖` — Claude is working
- `💬` — Claude is waiting for input

### Installation

```bash
$ claude plugin marketplace add max-sixty/worktrunk
$ claude plugin install worktrunk@worktrunk
```

### Manual status markers

Set status markers manually for any workflow:

```bash
$ wt config var set marker "🚧"                   # Current branch
$ wt config var set marker "✅" --branch feature  # Specific branch
$ git config worktrunk.marker.feature "💬"        # Direct git config
```

## Statusline

`wt list statusline --claude-code` outputs a single-line status for the Claude Code statusline (currently this grabs CI status, so is too slow to use in synchronous contexts—if a faster version would be helpful, please [open an issue](https://github.com/max-sixty/worktrunk/issues)):

`~/w/myproject.feature-auth  !🤖  @`<span style='color:#0a0'>+42</span> <span style='color:#a00'>-8</span>`  `<span style='color:#0a0'>↑3</span>`  `<span style='color:#0a0'>⇡1</span>`  `<span style='color:#0a0'>●</span>`  | Opus`

Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "wt list statusline --claude-code"
  }
}
```
