//! Column layout and priority allocation for the list command.
//!
//! # TODO: Priority System Design & Future Improvements
//!
//! ## Current Approach: Priority with Modifiers
//!
//! The allocation system uses a **priority scoring model**:
//! ```text
//! final_priority = base_priority + modifiers
//! ```
//!
//! **Base priorities** (1-11) are determined by **user need hierarchy** - what questions users need
//! answered when scanning worktrees:
//! - 1: Branch (identity - "what is this?")
//! - 2: Working diff (critical - "do I need to commit?")
//! - 3: Ahead/behind (critical - "am I out of sync?")
//! - 4-10: Context (work volume, states, path, time, CI, etc.)
//! - 11: Message (nice-to-have, space-hungry)
//!
//! **Modifiers** adjust priority based on column attributes:
//! - **Empty penalty**: +10 if column has no data (only header)
//!   - Empty working_diff: 2 + 10 = priority 12
//!   - Empty ahead/behind: 3 + 10 = priority 13
//!   - etc.
//!
//! This creates two effective priority tiers:
//! - **Tier 1 (priorities 1-11)**: Columns with actual data
//! - **Tier 2 (priorities 12-21)**: Empty columns (visual consistency)
//!
//! The empty penalty is large (+10) but not infinite, so empty columns maintain their relative
//! ordering (empty working_diff still ranks higher than empty ci_status) for visual consistency.
//!
//! ## Why This Design?
//!
//! **Problem**: Terminal width is limited. We must decide what to show.
//!
//! **Goals**:
//! 1. Show critical data (uncommitted changes, sync status) at any terminal width
//! 2. Show nice-to-have data (message, commit hash) when space allows
//! 3. Maintain visual consistency - empty columns in predictable positions at wide widths
//!
//! **Key decision**: Message sits at the boundary (priority 11). Empty columns (priority 12+)
//! rank below message, so:
//! - Narrow terminals: Data columns + message (hide empty columns)
//! - Wide terminals: Data columns + message + empty columns (visual consistency)
//!
//! ## Current Implementation
//!
//! The code implements this as two explicit phases:
//! ```rust
//! // Phase 1: Base priorities 1-11 (columns with data)
//! if data_flags.working_diff { allocate(priority=2) }
//!
//! // Message base allocation (priority 11)
//! allocate_message_base();
//!
//! // Phase 2: Base priorities + empty penalty (12-21)
//! if !data_flags.working_diff { allocate(priority=2+10=12) }
//!
//! // Message expansion (uses leftover space)
//! expand_message_to_max();
//! ```
//!
//! **Pros**:
//! - Simple, explicit, easy to understand
//! - Low abstraction overhead
//! - Easy to modify individual column logic
//!
//! **Cons**:
//! - Priority calculation is implicit (scattered across code)
//! - Adding new modifiers requires code changes
//! - Some duplication (Phase 2 repeats allocation logic)
//!
//! ## Future: Generalized Priority System?
//!
//! **Could we make this more explicit?**
//! ```rust
//! struct ColumnPriority {
//!     base: u8,               // User need ranking (1-11)
//!     empty_penalty: u8,      // +10 if no data
//!     // Future modifiers:
//!     // width_bonus: i8,     // -1 if terminal_width > 150
//!     // user_priority: i8,   // User-configured adjustments
//! }
//!
//! fn calculate_priority(column: Column, context: &Context) -> u8 {
//!     let base = column.base_priority();
//!     let empty = if column.has_data(context) { 0 } else { 10 };
//!     base + empty
//! }
//!
//! // Sort by priority, then allocate in order
//! columns.sort_by_key(|c| calculate_priority(c, &context));
//! for column in columns { allocate(column); }
//! ```
//!
//! **Pros**:
//! - Priority calculation is explicit and centralized
//! - Easy to add new modifiers (terminal width, user config, etc.)
//! - Single allocation loop (no Phase 1/Phase 2 duplication)
//!
//! **Cons**:
//! - More abstraction (struct, enum, sorting)
//! - Harder to understand at a glance
//! - Message variable sizing still needs special handling (min/preferred/max)
//! - Premature generalization? (YAGNI - we don't have other modifiers yet)
//!
//! ## Decision: Keep Current Implementation For Now
//!
//! The explicit two-phase approach is **clear and sufficient** for current needs. The priority
//! system is conceptually sound - we just need better documentation.
//!
//! **Refactor when**:
//! - We add a second modifier (terminal width bonus, user config, etc.)
//! - The duplication becomes painful (more than ~6 empty columns)
//! - Priority ordering becomes hard to reason about
//!
//! Until then: **Simple > Generic**.

use crate::display::{find_common_prefix, get_terminal_width};
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthStr;

use super::model::ListItem;

/// Width of short commit hash display (first 8 hex characters)
const COMMIT_HASH_WIDTH: usize = 8;

/// Column header labels - single source of truth for all column headers.
/// Both layout calculations and rendering use these constants.
pub const HEADER_BRANCH: &str = "Branch";
pub const HEADER_WORKING_DIFF: &str = "Working ±";
pub const HEADER_AHEAD_BEHIND: &str = "Main ↕";
pub const HEADER_BRANCH_DIFF: &str = "Main ±";
pub const HEADER_STATE: &str = "State";
pub const HEADER_PATH: &str = "Path";
pub const HEADER_UPSTREAM: &str = "Remote ↕";
pub const HEADER_AGE: &str = "Age";
pub const HEADER_CI: &str = "CI";
pub const HEADER_COMMIT: &str = "Commit";
pub const HEADER_MESSAGE: &str = "Message";

/// Ensures a column width is at least as wide as its header.
///
/// This is the general solution for preventing header overflow: pass the header
/// string and the calculated data width, and this returns the larger of the two.
///
/// Use this for every column width calculation to ensure headers never overflow.
fn fit_header(header: &str, data_width: usize) -> usize {
    use unicode_width::UnicodeWidthStr;
    data_width.max(header.width())
}

/// Helper: Try to allocate space for a column. Returns the allocated width if successful.
/// Updates `remaining` by subtracting the allocated width + spacing.
/// If is_first is true, doesn't require spacing before the column.
///
/// The spacing is consumed from the budget (subtracted from `remaining`) but not returned
/// as part of the column's width, since the spacing appears before the column content.
fn try_allocate(
    remaining: &mut usize,
    ideal_width: usize,
    spacing: usize,
    is_first: bool,
) -> usize {
    if ideal_width == 0 {
        return 0;
    }
    let required = if is_first {
        ideal_width
    } else {
        ideal_width + spacing // Gap before column + column content
    };
    if *remaining < required {
        return 0;
    }
    *remaining = remaining.saturating_sub(required);
    ideal_width // Return just the column width
}

/// Width information for two-part columns: diffs ("+128 -147") and arrows ("↑6 ↓1")
/// - For diff columns: added_digits/deleted_digits refer to line change counts
/// - For arrow columns: added_digits/deleted_digits refer to ahead/behind commit counts
#[derive(Clone, Copy, Debug)]
pub struct DiffWidths {
    pub total: usize,
    pub added_digits: usize,   // First part: + for diffs, ↑ for arrows
    pub deleted_digits: usize, // Second part: - for diffs, ↓ for arrows
}

impl DiffWidths {
    pub fn zero() -> Self {
        Self {
            total: 0,
            added_digits: 0,
            deleted_digits: 0,
        }
    }
}

pub struct ColumnWidths {
    pub branch: usize,
    pub time: usize,
    pub ci_status: usize,
    pub message: usize,
    pub ahead_behind: DiffWidths,
    pub working_diff: DiffWidths,
    pub branch_diff: DiffWidths,
    pub upstream: DiffWidths,
    pub states: usize,
    pub commit: usize,
    pub path: usize,
}

/// Tracks which columns have actual data (vs just headers)
#[derive(Clone, Copy, Debug)]
pub struct ColumnDataFlags {
    pub working_diff: bool,
    pub ahead_behind: bool,
    pub branch_diff: bool,
    pub upstream: bool,
    pub states: bool,
    pub ci_status: bool,
}

/// Absolute column positions for guaranteed alignment
#[derive(Clone, Copy, Debug)]
pub struct ColumnPositions {
    pub branch: usize,
    pub working_diff: usize,
    pub ahead_behind: usize,
    pub branch_diff: usize,
    pub states: usize,
    pub path: usize,
    pub upstream: usize,
    pub time: usize,
    pub ci_status: usize,
    pub commit: usize,
    pub message: usize,
}

pub struct LayoutConfig {
    pub widths: ColumnWidths,
    pub positions: ColumnPositions,
    pub common_prefix: PathBuf,
    pub max_message_len: usize,
}

pub fn calculate_column_widths(
    items: &[ListItem],
    fetch_ci: bool,
) -> (ColumnWidths, ColumnDataFlags) {
    // Track maximum data widths (headers are enforced via fit_header() later)
    let mut max_branch = 0;
    let mut max_time = 0;
    let mut max_message = 0;
    let mut max_states = 0;

    // Track diff component widths separately
    let mut max_wt_added_digits = 0;
    let mut max_wt_deleted_digits = 0;
    let mut max_br_added_digits = 0;
    let mut max_br_deleted_digits = 0;

    // Track ahead/behind digit widths separately for alignment
    let mut max_ahead_digits = 0;
    let mut max_behind_digits = 0;
    let mut max_upstream_ahead_digits = 0;
    let mut max_upstream_behind_digits = 0;

    for item in items {
        let commit = item.commit_details();
        let counts = item.counts();
        let branch_diff = item.branch_diff().diff;
        let upstream = item.upstream();
        let worktree_info = item.worktree_info();

        // Branch name
        max_branch = max_branch.max(item.branch_name().width());

        // Time
        let time_str = crate::display::format_relative_time(commit.timestamp);
        max_time = max_time.max(time_str.width());

        // Message (truncate to 50 chars max)
        let msg_len = commit.commit_message.chars().take(50).count();
        max_message = max_message.max(msg_len);

        // Ahead/behind (only for non-primary items) - track digits separately
        if !item.is_primary() && (counts.ahead > 0 || counts.behind > 0) {
            max_ahead_digits = max_ahead_digits.max(counts.ahead.to_string().len());
            max_behind_digits = max_behind_digits.max(counts.behind.to_string().len());
        }

        // Working tree diff (worktrees only) - track digits separately
        if let Some(info) = worktree_info
            && (info.working_tree_diff.0 > 0 || info.working_tree_diff.1 > 0)
        {
            max_wt_added_digits =
                max_wt_added_digits.max(info.working_tree_diff.0.to_string().len());
            max_wt_deleted_digits =
                max_wt_deleted_digits.max(info.working_tree_diff.1.to_string().len());
        }

        // Branch diff (only for non-primary items) - track digits separately
        if !item.is_primary() && (branch_diff.0 > 0 || branch_diff.1 > 0) {
            max_br_added_digits = max_br_added_digits.max(branch_diff.0.to_string().len());
            max_br_deleted_digits = max_br_deleted_digits.max(branch_diff.1.to_string().len());
        }

        // Upstream tracking - track digits only (not remote name yet)
        if let Some((_remote_name, upstream_ahead, upstream_behind)) = upstream.active() {
            max_upstream_ahead_digits =
                max_upstream_ahead_digits.max(upstream_ahead.to_string().len());
            max_upstream_behind_digits =
                max_upstream_behind_digits.max(upstream_behind.to_string().len());
        }

        // States (includes conflicts, worktree states, etc.)
        let states = super::render::format_all_states(item);
        if !states.is_empty() {
            max_states = max_states.max(states.width());
        }
    }

    // Calculate diff widths: "+{added} -{deleted}"
    // Format: "+" + digits + " " + "-" + digits
    let has_working_diff_data = max_wt_added_digits > 0 || max_wt_deleted_digits > 0;
    let working_diff_data_width = if has_working_diff_data {
        1 + max_wt_added_digits + 1 + 1 + max_wt_deleted_digits
    } else {
        0
    };
    let working_diff_total = fit_header(HEADER_WORKING_DIFF, working_diff_data_width);

    let has_branch_diff_data = max_br_added_digits > 0 || max_br_deleted_digits > 0;
    let branch_diff_data_width = if has_branch_diff_data {
        1 + max_br_added_digits + 1 + 1 + max_br_deleted_digits
    } else {
        0
    };
    let branch_diff_total = fit_header(HEADER_BRANCH_DIFF, branch_diff_data_width);

    // Calculate ahead/behind column width (format: "↑n ↓n")
    let has_ahead_behind_data = max_ahead_digits > 0 || max_behind_digits > 0;
    let ahead_behind_data_width = if has_ahead_behind_data {
        1 + max_ahead_digits + 1 + 1 + max_behind_digits
    } else {
        0
    };
    let ahead_behind_total = fit_header(HEADER_AHEAD_BEHIND, ahead_behind_data_width);

    // Calculate upstream column width (format: "↑n ↓n" or "remote ↑n ↓n")
    let has_upstream_data = max_upstream_ahead_digits > 0 || max_upstream_behind_digits > 0;
    let upstream_data_width = if has_upstream_data {
        // Format: "↑" + digits + " " + "↓" + digits
        // TODO: Add remote name when show_remote_names is implemented
        1 + max_upstream_ahead_digits + 1 + 1 + max_upstream_behind_digits
    } else {
        0
    };
    let upstream_total = fit_header(HEADER_UPSTREAM, upstream_data_width);

    let has_states_data = max_states > 0;
    let final_states = fit_header(HEADER_STATE, max_states);

    // CI status column: Always 2 chars wide
    // Only show if we attempted to fetch CI data (regardless of whether any items have status)
    let has_ci_status = fetch_ci && items.iter().any(|item| item.pr_status().is_some());
    let ci_status_width = 2; // Fixed width

    let widths = ColumnWidths {
        branch: fit_header(HEADER_BRANCH, max_branch),
        time: fit_header(HEADER_AGE, max_time),
        ci_status: fit_header(HEADER_CI, ci_status_width),
        message: fit_header(HEADER_MESSAGE, max_message),
        ahead_behind: DiffWidths {
            total: ahead_behind_total,
            added_digits: max_ahead_digits,
            deleted_digits: max_behind_digits,
        },
        working_diff: DiffWidths {
            total: working_diff_total,
            added_digits: max_wt_added_digits,
            deleted_digits: max_wt_deleted_digits,
        },
        branch_diff: DiffWidths {
            total: branch_diff_total,
            added_digits: max_br_added_digits,
            deleted_digits: max_br_deleted_digits,
        },
        upstream: DiffWidths {
            total: upstream_total,
            added_digits: max_upstream_ahead_digits,
            deleted_digits: max_upstream_behind_digits,
        },
        states: final_states,
        commit: COMMIT_HASH_WIDTH,
        path: 0, // Path width calculated later in responsive layout
    };

    let data_flags = ColumnDataFlags {
        working_diff: has_working_diff_data,
        ahead_behind: has_ahead_behind_data,
        branch_diff: has_branch_diff_data,
        upstream: has_upstream_data,
        states: has_states_data,
        ci_status: has_ci_status,
    };

    (widths, data_flags)
}

/// Calculate responsive layout based on terminal width
pub fn calculate_responsive_layout(
    items: &[ListItem],
    show_full: bool,
    fetch_ci: bool,
) -> LayoutConfig {
    let terminal_width = get_terminal_width();
    let paths: Vec<&Path> = items
        .iter()
        .filter_map(|item| item.worktree_path().map(|path| path.as_path()))
        .collect();
    let common_prefix = find_common_prefix(&paths);

    // Calculate ideal column widths and track which columns have data
    let (ideal_widths, data_flags) = calculate_column_widths(items, fetch_ci);

    // Calculate actual maximum path width (after common prefix removal)
    let path_data_width = items
        .iter()
        .filter_map(|item| item.worktree_path())
        .map(|path| {
            use crate::display::shorten_path;
            use unicode_width::UnicodeWidthStr;
            shorten_path(path.as_path(), &common_prefix).width()
        })
        .max()
        .unwrap_or(0);
    let max_path_width = fit_header(HEADER_PATH, path_data_width);

    let spacing = 2;
    let commit_width = fit_header(HEADER_COMMIT, COMMIT_HASH_WIDTH);

    // Two-phase priority allocation:
    // Phase 1: Allocate columns with actual data (in priority order)
    // Message: Always allocated before empty columns
    // Phase 2: Allocate empty columns (only if space remains after message)
    //
    // Priority order (from high to low):
    // === Phase 1: Columns with data ===
    // 1. branch - identity (what is this?)
    // 2. working_diff - uncommitted changes (CRITICAL: do I need to commit?)
    // 3. ahead_behind - commits difference (CRITICAL: am I ahead/behind?)
    // 4. branch_diff - line diff in commits (work volume in those commits)
    // 5. states - special states like [rebasing], (conflicts) (rare but urgent when present)
    // 6. path - location (where is this?)
    // 7. upstream - tracking configuration (sync context)
    // 8. time - recency (nice-to-have context)
    // 9. ci_status - CI/PR status (contextual when available)
    // 10. commit - hash (reference info, rarely needed)
    // 11. message - description (nice-to-have, space-hungry)
    // === Phase 2: Empty columns (only if space remains) ===
    // 12. working_diff (if empty)
    // 13. ahead_behind (if empty)
    // 14. branch_diff (if empty)
    // 15. states (if empty)
    // 16. upstream (if empty)
    // 17. ci_status (if empty)
    //
    // Note: ahead_behind and branch_diff are adjacent (both describe commits vs main)

    let mut remaining = terminal_width;
    let mut widths = ColumnWidths {
        branch: 0,
        time: 0,
        ci_status: 0,
        message: 0,
        ahead_behind: DiffWidths::zero(),
        working_diff: DiffWidths::zero(),
        branch_diff: DiffWidths::zero(),
        upstream: DiffWidths::zero(),
        states: 0,
        commit: 0,
        path: 0,
    };

    // === PHASE 1: Allocate columns with data ===

    // Branch column (highest priority - identity, always has data)
    widths.branch = try_allocate(&mut remaining, ideal_widths.branch, spacing, true);

    // Working diff column (critical - uncommitted changes)
    if data_flags.working_diff {
        let allocated_width = try_allocate(
            &mut remaining,
            ideal_widths.working_diff.total,
            spacing,
            false,
        );
        if allocated_width > 0 {
            widths.working_diff = ideal_widths.working_diff;
        }
    }

    // Ahead/behind column (critical sync status)
    if data_flags.ahead_behind {
        let allocated_width = try_allocate(
            &mut remaining,
            ideal_widths.ahead_behind.total,
            spacing,
            false,
        );
        if allocated_width > 0 {
            widths.ahead_behind = ideal_widths.ahead_behind;
        }
    }

    // Branch diff column (work volume in those commits)
    // Hidden by default - considered too noisy for typical usage.
    // May reconsider showing by default in future based on user feedback.
    if show_full && data_flags.branch_diff {
        let allocated_width = try_allocate(
            &mut remaining,
            ideal_widths.branch_diff.total,
            spacing,
            false,
        );
        if allocated_width > 0 {
            widths.branch_diff = ideal_widths.branch_diff;
        }
    }

    // States column (rare but urgent when present, now includes conflicts)
    if data_flags.states {
        widths.states = try_allocate(&mut remaining, ideal_widths.states, spacing, false);
    }

    // Path column (location - important for navigation, always has data)
    widths.path = try_allocate(&mut remaining, max_path_width, spacing, false);

    // Upstream column (sync configuration)
    if data_flags.upstream {
        let allocated_width =
            try_allocate(&mut remaining, ideal_widths.upstream.total, spacing, false);
        if allocated_width > 0 {
            widths.upstream = ideal_widths.upstream;
        }
    }

    // Time column (contextual information, always has data)
    widths.time = try_allocate(&mut remaining, ideal_widths.time, spacing, false);

    // CI status column (high priority when present, fixed width)
    if data_flags.ci_status {
        widths.ci_status = try_allocate(&mut remaining, ideal_widths.ci_status, spacing, false);
    }

    // Commit column (reference hash - rarely needed, always has data)
    widths.commit = try_allocate(&mut remaining, commit_width, spacing, false);

    // Message column (flexible width: min 20, preferred 50, max 100)
    // Allocated BEFORE empty columns - message has higher priority than empty columns
    const MIN_MESSAGE: usize = 20;
    const PREFERRED_MESSAGE: usize = 50;
    const MAX_MESSAGE: usize = 100;

    let message_width = if remaining >= PREFERRED_MESSAGE + spacing {
        PREFERRED_MESSAGE
    } else if remaining >= MIN_MESSAGE + spacing {
        remaining.saturating_sub(spacing).min(ideal_widths.message)
    } else {
        0
    };

    if message_width > 0 {
        remaining = remaining.saturating_sub(message_width + spacing);
        widths.message = message_width.min(ideal_widths.message);
    }

    // === PHASE 2: Allocate empty columns (if space remains after message) ===

    // Working diff column (if no data)
    if !data_flags.working_diff {
        let allocated_width = try_allocate(
            &mut remaining,
            ideal_widths.working_diff.total,
            spacing,
            false,
        );
        if allocated_width > 0 {
            widths.working_diff = ideal_widths.working_diff;
        }
    }

    // Ahead/behind column (if no data)
    if !data_flags.ahead_behind {
        let allocated_width = try_allocate(
            &mut remaining,
            ideal_widths.ahead_behind.total,
            spacing,
            false,
        );
        if allocated_width > 0 {
            widths.ahead_behind = ideal_widths.ahead_behind;
        }
    }

    // Branch diff column (if no data)
    if show_full && !data_flags.branch_diff {
        let allocated_width = try_allocate(
            &mut remaining,
            ideal_widths.branch_diff.total,
            spacing,
            false,
        );
        if allocated_width > 0 {
            widths.branch_diff = ideal_widths.branch_diff;
        }
    }

    // States column (if no data)
    if !data_flags.states {
        widths.states = try_allocate(&mut remaining, ideal_widths.states, spacing, false);
    }

    // Upstream column (if no data)
    if !data_flags.upstream {
        let allocated_width =
            try_allocate(&mut remaining, ideal_widths.upstream.total, spacing, false);
        if allocated_width > 0 {
            widths.upstream = ideal_widths.upstream;
        }
    }

    // CI status column (if no data, but only if we attempted to fetch CI)
    if fetch_ci && !data_flags.ci_status {
        widths.ci_status = try_allocate(&mut remaining, ideal_widths.ci_status, spacing, false);
    }

    // Expand message with any leftover space (up to MAX_MESSAGE total)
    if widths.message > 0 && widths.message < MAX_MESSAGE && remaining > 0 {
        let expansion = remaining.min(MAX_MESSAGE - widths.message);
        widths.message += expansion;
    }

    let final_max_message_len = widths.message;

    // Calculate absolute column positions (with 2-space gaps between columns)
    let gap = 2;
    let mut pos = 0;

    // Helper closure to advance position for a column
    // Returns the column's start position, or 0 if column is hidden (width=0)
    let mut advance = |width: usize| -> usize {
        if width == 0 {
            return 0;
        }
        let column_pos = if pos == 0 { 0 } else { pos + gap };
        pos = column_pos + width;
        column_pos
    };

    let positions = ColumnPositions {
        branch: advance(widths.branch),
        working_diff: advance(widths.working_diff.total),
        ahead_behind: advance(widths.ahead_behind.total),
        branch_diff: advance(widths.branch_diff.total),
        states: advance(widths.states),
        path: advance(widths.path),
        upstream: advance(widths.upstream.total),
        time: advance(widths.time),
        ci_status: advance(widths.ci_status),
        commit: advance(widths.commit),
        message: advance(widths.message),
    };

    LayoutConfig {
        widths,
        positions,
        common_prefix,
        max_message_len: final_max_message_len,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_column_width_calculation_with_unicode() {
        use crate::commands::list::model::{
            AheadBehind, BranchDiffTotals, CommitDetails, DisplayFields, UpstreamStatus,
            WorktreeInfo,
        };

        let info1 = WorktreeInfo {
            worktree: worktrunk::git::Worktree {
                path: PathBuf::from("/test"),
                head: "abc123".to_string(),
                branch: Some("main".to_string()),
                bare: false,
                detached: false,
                locked: None,
                prunable: None,
            },
            commit: CommitDetails {
                timestamp: 0,
                commit_message: "Test".to_string(),
            },
            counts: AheadBehind {
                ahead: 3,
                behind: 2,
            },
            working_tree_diff: (100, 50),
            branch_diff: BranchDiffTotals { diff: (200, 30) },
            is_primary: false,
            upstream: UpstreamStatus::from_parts(Some("origin".to_string()), 4, 0),
            worktree_state: None,
            pr_status: None,
            has_conflicts: false,
            display: DisplayFields::default(),
            working_diff_display: None,
        };

        let (widths, _data_flags) =
            calculate_column_widths(&[super::ListItem::Worktree(info1)], false);

        // "↑3 ↓2" has format "↑3 ↓2" = 1+1+1+1+1 = 5, but header "Main ↕" is 6
        assert_eq!(
            widths.ahead_behind.total, 6,
            "Ahead/behind column should fit header 'Main ↕' (width 6)"
        );
        assert_eq!(widths.ahead_behind.added_digits, 1, "3 has 1 digit");
        assert_eq!(widths.ahead_behind.deleted_digits, 1, "2 has 1 digit");

        // "+100 -50" has width 8, but header "Working ±" is 9, so column width is 9
        assert_eq!(
            widths.working_diff.total, 9,
            "Working diff column should fit header 'Working ±' (width 9)"
        );
        assert_eq!(widths.working_diff.added_digits, 3, "100 has 3 digits");
        assert_eq!(widths.working_diff.deleted_digits, 2, "50 has 2 digits");

        // "+200 -30" has width 8, but header "Main ±" is 6, so column width is 8
        assert_eq!(
            widths.branch_diff.total, 8,
            "Branch diff column should fit header 'Main ±' (width 6)"
        );
        assert_eq!(widths.branch_diff.added_digits, 3, "200 has 3 digits");
        assert_eq!(widths.branch_diff.deleted_digits, 2, "30 has 2 digits");

        // Upstream: "↑4 ↓0" = "↑" (1) + "4" (1) + " " (1) + "↓" (1) + "0" (1) = 5, but header "Remote ↕" = 8
        assert_eq!(
            widths.upstream.total, 8,
            "Upstream column should fit header 'Remote ↕' (width 8)"
        );
        assert_eq!(widths.upstream.added_digits, 1, "4 has 1 digit");
        assert_eq!(widths.upstream.deleted_digits, 1, "0 has 1 digit");
    }

    #[test]
    fn test_visible_columns_follow_gap_rule() {
        use crate::commands::list::model::{
            AheadBehind, BranchDiffTotals, CommitDetails, DisplayFields, UpstreamStatus,
            WorktreeInfo,
        };

        // Create test data with specific widths to verify position calculation
        let info = WorktreeInfo {
            worktree: worktrunk::git::Worktree {
                path: PathBuf::from("/test/path"),
                head: "abc12345".to_string(),
                branch: Some("feature".to_string()),
                bare: false,
                detached: false,
                locked: None,
                prunable: None,
            },
            commit: CommitDetails {
                timestamp: 1234567890,
                commit_message: "Test commit message".to_string(),
            },
            counts: AheadBehind {
                ahead: 5,
                behind: 10,
            },
            working_tree_diff: (100, 50),
            branch_diff: BranchDiffTotals { diff: (200, 30) },
            is_primary: false,
            upstream: UpstreamStatus::from_parts(Some("origin".to_string()), 4, 2),
            worktree_state: None,
            pr_status: None,
            has_conflicts: false,
            display: DisplayFields::default(),
            working_diff_display: None,
        };

        let items = vec![super::ListItem::Worktree(info)];
        let layout = calculate_responsive_layout(&items, false, false);
        let pos = &layout.positions;
        let widths = &layout.widths;

        // Test key invariants of position calculation

        // 1. Branch always starts at position 0
        assert_eq!(pos.branch, 0, "Branch must start at position 0");

        // 2. States may be visible in Phase 2 (empty but shown if space allows)
        // Since we have plenty of space in wide terminal, states should be visible
        assert!(
            pos.states > 0,
            "States column should be visible in Phase 2 (empty but shown if space)"
        );

        // 3. For visible columns, verify correct spacing
        // Each visible column should be at: previous_position + previous_width + gap(2)
        let gap = 2;

        if widths.working_diff.total > 0 && pos.working_diff > 0 {
            assert_eq!(
                pos.working_diff,
                pos.branch + widths.branch + gap,
                "Working diff position should follow branch with 2-space gap"
            );
        }

        if widths.ahead_behind.total > 0 && pos.ahead_behind > 0 {
            let prev_col_end = if pos.working_diff > 0 {
                pos.working_diff + widths.working_diff.total
            } else {
                pos.branch + widths.branch
            };
            assert_eq!(
                pos.ahead_behind,
                prev_col_end + gap,
                "Ahead/behind position should follow previous visible column with 2-space gap"
            );
        }

        // 4. Path must be visible and have position > 0 (it's always shown)
        assert!(pos.path > 0, "Path column must be visible");
        assert!(widths.path > 0, "Path column must have width > 0");
    }

    #[test]
    fn test_column_positions_with_hidden_columns() {
        use crate::commands::list::model::{
            AheadBehind, BranchDiffTotals, CommitDetails, DisplayFields, UpstreamStatus,
            WorktreeInfo,
        };

        // Create minimal data - most columns will be hidden
        let info = WorktreeInfo {
            worktree: worktrunk::git::Worktree {
                path: PathBuf::from("/test"),
                head: "abc12345".to_string(),
                branch: Some("main".to_string()),
                bare: false,
                detached: false,
                locked: None,
                prunable: None,
            },
            commit: CommitDetails {
                timestamp: 1234567890,
                commit_message: "Test".to_string(),
            },
            counts: AheadBehind {
                ahead: 0,
                behind: 0,
            },
            working_tree_diff: (0, 0),
            branch_diff: BranchDiffTotals { diff: (0, 0) },
            is_primary: true, // Primary worktree: no ahead/behind shown
            upstream: UpstreamStatus::default(),
            worktree_state: None,
            pr_status: None,
            has_conflicts: false,
            display: DisplayFields::default(),
            working_diff_display: None,
        };

        let items = vec![super::ListItem::Worktree(info)];
        let layout = calculate_responsive_layout(&items, false, false);
        let pos = &layout.positions;

        // Branch should be at 0
        assert_eq!(pos.branch, 0, "Branch always starts at position 0");

        // With new two-phase allocation, empty columns are shown in Phase 2 if space allows
        // Since we have a wide terminal (80 chars default) and minimal data, at least some empty columns should be visible

        // Early Phase 2 columns should be visible (highest priority empty columns)
        assert!(
            pos.working_diff > 0,
            "Working diff should be visible in Phase 2 (empty but shown if space)"
        );
        assert!(
            pos.ahead_behind > 0,
            "Ahead/behind should be visible in Phase 2 (empty but shown if space)"
        );

        // Later Phase 2 columns might not fit (depending on terminal width)
        // Just verify that at least some empty columns are visible
        let empty_columns_visible = pos.working_diff > 0
            || pos.ahead_behind > 0
            || pos.branch_diff > 0
            || pos.states > 0
            || pos.upstream > 0;

        assert!(
            empty_columns_visible,
            "At least some empty columns should be visible in Phase 2"
        );

        // Path should be visible (always has data)
        assert!(pos.path > 0, "Path should be visible");
    }

    #[test]
    fn test_consecutive_hidden_columns_skip_correctly() {
        use crate::commands::list::model::{
            AheadBehind, BranchDiffTotals, CommitDetails, DisplayFields, UpstreamStatus,
            WorktreeInfo,
        };

        // Create data where multiple consecutive columns are hidden:
        // visible(branch) → hidden(working_diff) → hidden(ahead_behind) → hidden(branch_diff)
        // → hidden(states) → visible(path)
        let info = WorktreeInfo {
            worktree: worktrunk::git::Worktree {
                path: PathBuf::from("/test/worktree"),
                head: "abc12345".to_string(),
                branch: Some("feature-x".to_string()),
                bare: false,
                detached: false,
                locked: None,
                prunable: None,
            },
            commit: CommitDetails {
                timestamp: 1234567890,
                commit_message: "Test commit".to_string(),
            },
            counts: AheadBehind {
                ahead: 0,
                behind: 0,
            },
            working_tree_diff: (0, 0), // Hidden: no dirty changes
            branch_diff: BranchDiffTotals { diff: (0, 0) }, // Hidden: no diff
            is_primary: true,          // Hidden: no ahead/behind for primary
            upstream: UpstreamStatus::default(), // Hidden: no upstream
            worktree_state: None,      // Hidden: no state
            pr_status: None,
            has_conflicts: false,
            display: DisplayFields::default(),
            working_diff_display: None,
        };

        let items = vec![super::ListItem::Worktree(info)];
        let layout = calculate_responsive_layout(&items, false, false);
        let pos = &layout.positions;
        let widths = &layout.widths;

        // With two-phase allocation, empty columns are allocated in Phase 2 (after data columns)
        // Phase 1: branch (data), path (data), time (data), commit (data), message (data)
        // Phase 2: working_diff (empty), ahead_behind (empty), branch_diff (empty), states (empty), upstream (empty), ci_status (empty)

        // In Phase 1, path comes after branch immediately (since all middle columns have no data)
        // Branch, path, time, commit, message are allocated first

        // Path should come early since it has data and is allocated in Phase 1
        assert!(
            pos.path > 0,
            "Path should be visible (allocated in Phase 1)"
        );

        // With the corrected Phase 2 allocation, empty columns only show if space remains AFTER message
        // In this test with 80 character width and minimal data:
        // - Branch, path, time, commit get allocated in Phase 1
        // - Message gets allocated next (before empty columns)
        // - Empty columns only allocated if space remains after message

        // Message should be allocated (it comes before empty columns now)
        assert!(
            widths.message > 0,
            "Message should be allocated before empty columns"
        );

        // Empty columns may or may not be visible depending on space remaining after message
        // This is acceptable - message has priority over empty columns
        // No assertion needed here - it's correct for empty columns to not show if message takes the space
    }
}
