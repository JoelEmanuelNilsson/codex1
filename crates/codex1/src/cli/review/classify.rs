//! Review-record freshness classification.
//!
//! See `docs/cli-contract-schemas.md` § "Review record freshness". The
//! classification is pure over `MissionState` — no IO, no mutation.

use crate::state::schema::{MissionState, ReviewRecordCategory, ReviewVerdict, TaskStatus};

/// Inputs to the classifier. `state_revision_at_record` is the pre-bump
/// revision observed at the start of `review record`'s mutation closure.
pub struct ClassifyInput<'a> {
    pub state: &'a MissionState,
    pub review_task_id: &'a str,
    pub target_task_ids: &'a [String],
    /// The mutation-closure-entry revision. `review record` runs inside
    /// `state::mutate`; at closure entry `state.revision` is pre-bump.
    pub state_revision_at_record: u64,
}

/// Classify a review record per the freshness rules.
///
/// Order (per handoff spec):
/// 1. `close.terminal_at` set          -> `contaminated_after_terminal`.
/// 2. Review task or any target has `superseded_by` set OR the target's
///    status is `Superseded`                            -> `stale_superseded`.
/// 3. No prior `review.started` record found            -> `accepted_current`
///    (this is a first-time record; the boundary is "now"). Classification
///    fall-through handles this by returning `accepted_current`.
/// 4. `state_revision_at_record > boundary_revision`    -> `late_same_boundary`.
/// 5. else                                              -> `accepted_current`.
#[must_use]
pub fn classify(input: &ClassifyInput<'_>) -> ReviewRecordCategory {
    if input.state.close.terminal_at.is_some() {
        return ReviewRecordCategory::ContaminatedAfterTerminal;
    }
    if is_superseded(input) {
        return ReviewRecordCategory::StaleSuperseded;
    }

    // Pending record tells us the boundary revision the review was started
    // at. If there is no start record, classify as `accepted_current` and
    // let the record command create one.
    let boundary = input
        .state
        .reviews
        .get(input.review_task_id)
        .map(|r| r.boundary_revision);
    match boundary {
        Some(b) if input.state_revision_at_record > b => ReviewRecordCategory::LateSameBoundary,
        _ => ReviewRecordCategory::AcceptedCurrent,
    }
}

fn is_superseded(input: &ClassifyInput<'_>) -> bool {
    if let Some(review) = input.state.tasks.get(input.review_task_id) {
        if review.superseded_by.is_some() || matches!(review.status, TaskStatus::Superseded) {
            return true;
        }
    }
    for tid in input.target_task_ids {
        if let Some(t) = input.state.tasks.get(tid) {
            if t.superseded_by.is_some() || matches!(t.status, TaskStatus::Superseded) {
                return true;
            }
        }
    }
    false
}

/// Human-friendly category label for event payloads.
#[must_use]
pub fn category_str(cat: ReviewRecordCategory) -> &'static str {
    match cat {
        ReviewRecordCategory::AcceptedCurrent => "accepted_current",
        ReviewRecordCategory::LateSameBoundary => "late_same_boundary",
        ReviewRecordCategory::StaleSuperseded => "stale_superseded",
        ReviewRecordCategory::ContaminatedAfterTerminal => "contaminated_after_terminal",
    }
}

/// Stable string label for a review verdict.
#[must_use]
pub fn verdict_str(v: &ReviewVerdict) -> &'static str {
    match v {
        ReviewVerdict::Pending => "pending",
        ReviewVerdict::Clean => "clean",
        ReviewVerdict::Dirty => "dirty",
    }
}
