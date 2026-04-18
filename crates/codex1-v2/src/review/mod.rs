//! Review bundles, reviewer outputs, and cleanliness computation.
//!
//! The review contract is the retrospective's most important remedy: no
//! parent self-review, every reviewer output binds to the truth it was
//! produced against, and any P0/P1/P2 finding keeps the bundle dirty.

// Non-test callers land in T21/T22 (review CLI commands).
#![allow(dead_code)]

pub(crate) mod bundle;
pub(crate) mod clean;
pub(crate) mod output;

// Re-exports consumed by T21/T22 review CLI commands and by test helpers.
#[allow(unused_imports)]
pub use bundle::{ReviewBundle, ReviewRequirement, ReviewStatus, ReviewTarget};
#[allow(unused_imports)]
pub use clean::{compute_cleanliness, CleanlinessVerdict, CurrentTruth};
#[allow(unused_imports)]
pub use output::{Finding, FindingSeverity, ReviewerOutput, ReviewerResultKind};

/// Conventional directory for bundles inside a mission.
pub const BUNDLES_DIRNAME: &str = "reviews";
/// Conventional directory for reviewer outputs inside a mission.
pub const OUTPUTS_DIRNAME: &str = "reviews/outputs";
