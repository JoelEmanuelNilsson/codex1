//! Pure helpers for the replan trigger rule.
//!
//! The only automatic replan trigger is "six consecutive dirty reviews
//! on the same active target". `cli-review` increments the per-target
//! counter; this module reads it.

use crate::state::schema::{MissionState, TaskId};

/// Minimum consecutive-dirty count that forces a replan.
pub const DIRTY_THRESHOLD: u32 = 6;

/// Allowed `--reason` codes for `replan record`.
pub const ALLOWED_REASONS: &[&str] = &[
    "six_dirty",
    "scope_change",
    "architecture_shift",
    "risk_discovered",
    "user_request",
];

/// Find the first target whose consecutive-dirty counter has reached the
/// threshold. Returns the target id and its counter value.
#[must_use]
pub fn breach(state: &MissionState) -> Option<(TaskId, u32)> {
    state
        .replan
        .consecutive_dirty_by_target
        .iter()
        .find(|(_, &count)| count >= DIRTY_THRESHOLD)
        .map(|(id, &count)| (id.clone(), count))
}
