//! Envelope emitters for the `outcome` commands.
//!
//! Foundation's `cli::dispatch` always prints the error envelope produced
//! by `CliError::to_envelope()`, and the `OutcomeIncomplete` variant has
//! no fields for `missing_fields` / `placeholders`. To honour the CLI
//! contract's `context.missing_fields` / `context.placeholders` promise
//! without touching Foundation files, the `check` and `ratify` handlers
//! assemble the enriched envelope themselves, print it, and exit with
//! the handled-error exit code before control returns to dispatch.

use serde_json::json;

use crate::core::envelope::JsonErr;
use crate::core::error::CliError;

/// Print the `OUTCOME_INCOMPLETE` envelope with `context.missing_fields`
/// and `context.placeholders` filled in, then exit with status 1.
///
/// Using `process::exit(1)` here is a deliberate escape hatch: it matches
/// the `ExitKind::HandledError` mapping for `OutcomeIncomplete` in
/// `lib::run`, and it prevents `cli::dispatch` from double-printing a
/// context-less envelope on top of ours.
pub fn emit_outcome_incomplete(
    mission_id: &str,
    missing_fields: Vec<String>,
    placeholders: Vec<String>,
) -> ! {
    let err = CliError::OutcomeIncomplete {
        message: build_message(&missing_fields, &placeholders),
        hint: Some(
            "Fill every required field and remove all `[codex1-fill:...]` markers, \
            TODO/TBD values, and vague 'works well' style entries."
                .to_string(),
        ),
    };
    let base = err.to_envelope();
    let context = json!({
        "mission_id": mission_id,
        "missing_fields": missing_fields,
        "placeholders": placeholders,
    });
    let enriched = JsonErr {
        ok: false,
        code: base.code,
        message: base.message,
        hint: base.hint,
        retryable: base.retryable,
        context,
    };
    println!("{}", enriched.to_pretty());
    std::process::exit(1);
}

fn build_message(missing: &[String], placeholders: &[String]) -> String {
    match (missing.len(), placeholders.len()) {
        (0, p) => format!("{p} placeholder(s) remain."),
        (m, 0) => format!("{m} required field(s) missing."),
        (m, p) => format!("{m} required field(s) missing and {p} placeholder(s) remain."),
    }
}
