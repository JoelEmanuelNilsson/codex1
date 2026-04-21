//! OUTCOME.md validation shared by `outcome check` and `outcome ratify`.
//!
//! Parses YAML frontmatter, checks required fields, rejects boilerplate,
//! and flags any `[codex1-fill:…]` markers anywhere in the file.

use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::core::error::CliError;

/// Result of validating OUTCOME.md. `ratifiable` is true iff both
/// `missing_fields` and `placeholders` are empty.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub ratifiable: bool,
    pub missing_fields: Vec<String>,
    pub placeholders: Vec<String>,
    pub frontmatter_raw: String,
    pub body: String,
}

/// Load and validate OUTCOME.md at `path`. Returns a structured report
/// even when invalid — the caller decides whether to error.
pub fn validate_outcome(
    path: &Path,
    expected_mission_id: &str,
) -> Result<ValidationReport, CliError> {
    if !path.is_file() {
        return Err(CliError::OutcomeIncomplete {
            message: format!("OUTCOME.md not found at {}", path.display()),
            hint: Some("Run `codex1 init --mission <id>` first.".to_string()),
        });
    }
    let raw = std::fs::read_to_string(path)?;
    let (frontmatter_raw, body) =
        split_frontmatter(&raw).ok_or_else(|| CliError::OutcomeIncomplete {
            message: "OUTCOME.md is missing YAML frontmatter (expected `---` fences)".to_string(),
            hint: Some("Start the file with `---`, a YAML block, and a closing `---`.".to_string()),
        })?;

    let mapping: Mapping = match serde_yaml::from_str::<Value>(&frontmatter_raw) {
        Ok(Value::Mapping(m)) => m,
        Ok(_) => {
            return Err(CliError::OutcomeIncomplete {
                message: "OUTCOME.md frontmatter must be a YAML mapping".to_string(),
                hint: None,
            });
        }
        Err(err) => {
            return Err(CliError::OutcomeIncomplete {
                message: format!("Failed to parse OUTCOME.md frontmatter: {err}"),
                hint: Some(
                    "Ensure the YAML between `---` fences is syntactically valid.".to_string(),
                ),
            });
        }
    };

    let mut missing_fields = Vec::new();
    let mut placeholders = Vec::new();

    check_scalar(
        &mapping,
        "mission_id",
        &mut missing_fields,
        &mut placeholders,
    );
    if let Some(actual) = mapping
        .get(Value::String("mission_id".to_string()))
        .and_then(Value::as_str)
        .map(str::trim)
    {
        if !actual.is_empty() && actual != expected_mission_id {
            missing_fields.push(format!(
                "mission_id (expected `{expected_mission_id}`, found `{actual}`)"
            ));
        }
    }
    check_scalar(&mapping, "status", &mut missing_fields, &mut placeholders);
    check_status_value(&mapping, &mut missing_fields);
    reject_forbidden_fields(&mapping, &mut missing_fields);
    check_scalar(&mapping, "title", &mut missing_fields, &mut placeholders);
    check_scalar(
        &mapping,
        "original_user_goal",
        &mut missing_fields,
        &mut placeholders,
    );
    check_scalar(
        &mapping,
        "interpreted_destination",
        &mut missing_fields,
        &mut placeholders,
    );

    check_non_empty_list(
        &mapping,
        "must_be_true",
        &mut missing_fields,
        &mut placeholders,
    );
    check_non_empty_list(
        &mapping,
        "success_criteria",
        &mut missing_fields,
        &mut placeholders,
    );

    check_non_empty_list(
        &mapping,
        "non_goals",
        &mut missing_fields,
        &mut placeholders,
    );
    check_non_empty_list(
        &mapping,
        "constraints",
        &mut missing_fields,
        &mut placeholders,
    );
    check_mapping_present(
        &mapping,
        "definitions",
        &mut missing_fields,
        &mut placeholders,
    );
    check_non_empty_list(
        &mapping,
        "quality_bar",
        &mut missing_fields,
        &mut placeholders,
    );
    check_non_empty_list(
        &mapping,
        "proof_expectations",
        &mut missing_fields,
        &mut placeholders,
    );
    check_non_empty_list(
        &mapping,
        "review_expectations",
        &mut missing_fields,
        &mut placeholders,
    );
    check_non_empty_list(
        &mapping,
        "known_risks",
        &mut missing_fields,
        &mut placeholders,
    );
    check_resolved_questions(&mapping, &mut missing_fields, &mut placeholders);

    // Stricter rejection: success_criteria entries must not be pure
    // outcome-grade boilerplate ("works well", "reliable", …).
    reject_success_criteria_boilerplate(&mapping, &mut placeholders);

    // Sweep the whole file for remaining fill markers, including ones
    // hiding in the body text.
    collect_fill_markers(&raw, &mut placeholders);

    // De-duplicate while preserving order.
    dedup_in_place(&mut missing_fields);
    dedup_in_place(&mut placeholders);

    Ok(ValidationReport {
        ratifiable: missing_fields.is_empty() && placeholders.is_empty(),
        missing_fields,
        placeholders,
        frontmatter_raw,
        body,
    })
}

/// Split OUTCOME.md into `(frontmatter_body, markdown_body)`. Returns
/// `None` if the file lacks the standard `---…---` fences.
pub fn split_frontmatter(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    let rest = trimmed.strip_prefix("---")?;
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))?;
    // Find the next line that is exactly `---` (trailing whitespace ok).
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let line_end = offset + line.len();
        let stripped = line.trim_end_matches(['\n', '\r']);
        if stripped.trim_end() == "---" {
            let frontmatter = rest[..offset].to_string();
            let body_start = line_end;
            let body = rest.get(body_start..).unwrap_or("").to_string();
            return Some((frontmatter, body));
        }
        offset = line_end;
    }
    None
}

fn check_scalar(
    m: &Mapping,
    field: &str,
    missing: &mut Vec<String>,
    placeholders: &mut Vec<String>,
) {
    match m.get(Value::String(field.to_string())) {
        None | Some(Value::Null) => missing.push(field.to_string()),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                missing.push(field.to_string());
            } else if is_placeholder_value(trimmed) {
                placeholders.push(format!("{field}: {}", summarize(trimmed)));
            } else if let Some(marker) = find_fill_marker(trimmed) {
                placeholders.push(format!("{field}: {marker}"));
            }
        }
        Some(_) => {
            // status must be a string per contract; treat non-string scalars
            // as a missing-field problem.
            missing.push(format!("{field} (not a string)"));
        }
    }
}

fn check_non_empty_list(
    m: &Mapping,
    field: &str,
    missing: &mut Vec<String>,
    placeholders: &mut Vec<String>,
) {
    match m.get(Value::String(field.to_string())) {
        None => missing.push(field.to_string()),
        Some(Value::Sequence(seq)) => {
            if seq.is_empty() {
                missing.push(format!("{field} (empty list)"));
            } else {
                scan_list_entries(field, seq, missing, placeholders);
            }
        }
        Some(Value::Null) => missing.push(field.to_string()),
        Some(_) => missing.push(format!("{field} (not a list)")),
    }
}

fn check_status_value(m: &Mapping, missing: &mut Vec<String>) {
    let Some(Value::String(status)) = m.get(Value::String("status".to_string())) else {
        return;
    };
    let trimmed = status.trim();
    if trimmed.is_empty() {
        return;
    }
    if !matches!(trimmed, "draft" | "ratified") {
        missing.push(format!(
            "status (expected `draft` or `ratified`, found `{trimmed}`)"
        ));
    }
}

fn reject_forbidden_fields(m: &Mapping, missing: &mut Vec<String>) {
    for field in ["approval_boundaries", "autonomy"] {
        if m.contains_key(Value::String(field.to_string())) {
            missing.push(format!("{field} (forbidden in OUTCOME.md)"));
        }
    }
}

fn check_mapping_present(
    m: &Mapping,
    field: &str,
    missing: &mut Vec<String>,
    placeholders: &mut Vec<String>,
) {
    match m.get(Value::String(field.to_string())) {
        None | Some(Value::Null) => missing.push(field.to_string()),
        Some(Value::Mapping(map)) => {
            if map.is_empty() {
                missing.push(format!("{field} (empty mapping)"));
            }
            for (k, v) in map {
                match k.as_str().map(str::trim) {
                    Some("") => missing.push(format!("{field} (empty key)")),
                    Some(s) => {
                        if let Some(marker) = find_fill_marker(s) {
                            placeholders.push(format!("{field}: {marker}"));
                        }
                    }
                    None => missing.push(format!("{field} (key not a string)")),
                }
                match v {
                    Value::String(s) if !s.trim().is_empty() => {
                        scan_value_for_placeholders(field, v, placeholders);
                    }
                    Value::String(_) => missing.push(format!("{field} (empty value)")),
                    _ => missing.push(format!("{field} (value not a string)")),
                }
            }
        }
        Some(_) => missing.push(format!("{field} (not a mapping)")),
    }
}

fn check_resolved_questions(
    m: &Mapping,
    missing: &mut Vec<String>,
    placeholders: &mut Vec<String>,
) {
    let field = "resolved_questions";
    match m.get(Value::String(field.to_string())) {
        None | Some(Value::Null) => missing.push(field.to_string()),
        Some(Value::Sequence(seq)) => {
            if seq.is_empty() {
                missing.push(format!("{field} (empty list)"));
            }
            for (idx, entry) in seq.iter().enumerate() {
                match entry {
                    Value::Mapping(map) => {
                        let q = map.get(Value::String("question".to_string()));
                        let a = map.get(Value::String("answer".to_string()));
                        if !is_non_empty_string(q) {
                            missing.push(format!("{field}[{idx}].question"));
                        }
                        if !is_non_empty_string(a) {
                            missing.push(format!("{field}[{idx}].answer"));
                        }
                        scan_value_for_placeholders(field, entry, placeholders);
                    }
                    _ => missing.push(format!("{field}[{idx}] (not a question/answer mapping)")),
                }
            }
        }
        Some(_) => missing.push(format!("{field} (not a list)")),
    }
}

fn is_non_empty_string(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::String(s)) if !s.trim().is_empty())
}

fn scan_value_for_placeholders(field: &str, value: &Value, placeholders: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            let trimmed = s.trim();
            if is_placeholder_value(trimmed) {
                placeholders.push(format!("{field}: {}", summarize(trimmed)));
            } else if let Some(marker) = find_fill_marker(trimmed) {
                placeholders.push(format!("{field}: {marker}"));
            }
        }
        Value::Sequence(seq) => {
            for v in seq {
                scan_value_for_placeholders(field, v, placeholders);
            }
        }
        Value::Mapping(map) => {
            for v in map.values() {
                scan_value_for_placeholders(field, v, placeholders);
            }
        }
        _ => {}
    }
}

fn scan_list_entries(
    field: &str,
    seq: &[Value],
    missing: &mut Vec<String>,
    placeholders: &mut Vec<String>,
) {
    for (idx, entry) in seq.iter().enumerate() {
        let Value::String(s) = entry else {
            missing.push(format!("{field}[{idx}] (not a string)"));
            continue;
        };
        let trimmed = s.trim();
        if trimmed.is_empty() {
            missing.push(format!("{field}[{idx}] (empty string)"));
            continue;
        }
        if is_placeholder_value(trimmed) {
            placeholders.push(format!("{field}[{idx}]: {}", summarize(trimmed)));
        } else if let Some(marker) = find_fill_marker(trimmed) {
            placeholders.push(format!("{field}[{idx}]: {marker}"));
        }
    }
}

fn reject_success_criteria_boilerplate(m: &Mapping, placeholders: &mut Vec<String>) {
    let Some(Value::Sequence(seq)) = m.get(Value::String("success_criteria".to_string())) else {
        return;
    };
    for (idx, entry) in seq.iter().enumerate() {
        let Value::String(s) = entry else { continue };
        let lower = s.trim().to_lowercase();
        if is_success_boilerplate(&lower) {
            placeholders.push(format!("success_criteria[{idx}]: {}", summarize(s.trim())));
        }
    }
}

fn is_success_boilerplate(lower: &str) -> bool {
    matches!(
        lower,
        "works well"
            | "is reliable"
            | "reliable"
            | "is done"
            | "done"
            | "succeeds"
            | "success"
            | "overall"
            | "codex1 works well."
            | "codex1 works well"
    )
}

/// Return true for generic placeholder scalars that should block ratification.
fn is_placeholder_value(value: &str) -> bool {
    let lower = value.to_lowercase();
    matches!(
        lower.as_str(),
        "todo"
            | "tbd"
            | "..."
            | "\"works well\""
            | "works well"
            | "reliable"
            | "overall"
            | "codex1 works well."
            | "codex1 works well"
    )
}

fn find_fill_marker(value: &str) -> Option<String> {
    let needle = "[codex1-fill:";
    let start = value.find(needle)?;
    let rest = &value[start..];
    let end = rest.find(']')?;
    Some(rest[..=end].to_string())
}

fn collect_fill_markers(raw: &str, placeholders: &mut Vec<String>) {
    let mut cursor = 0;
    while let Some(rel) = raw[cursor..].find("[codex1-fill:") {
        let start = cursor + rel;
        if let Some(rel_end) = raw[start..].find(']') {
            let marker = &raw[start..=start + rel_end];
            placeholders.push(marker.to_string());
            cursor = start + rel_end + 1;
        } else {
            // Unterminated marker — record what we can and stop.
            placeholders.push(raw[start..].to_string());
            break;
        }
    }
}

fn dedup_in_place(items: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|s| seen.insert(s.clone()));
}

fn summarize(value: &str) -> String {
    const MAX: usize = 80;
    if value.chars().count() > MAX {
        let truncated: String = value.chars().take(MAX).collect();
        format!("{truncated}…")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_basic() {
        let raw = "---\nmission_id: demo\n---\n\n# Body\n";
        let (front, body) = split_frontmatter(raw).unwrap();
        assert_eq!(front, "mission_id: demo\n");
        assert_eq!(body, "\n# Body\n");
    }

    #[test]
    fn split_frontmatter_missing_fences_returns_none() {
        let raw = "# just markdown\n";
        assert!(split_frontmatter(raw).is_none());
    }

    #[test]
    fn find_fill_marker_detects_codex1_fill() {
        assert_eq!(
            find_fill_marker("[codex1-fill:title]").as_deref(),
            Some("[codex1-fill:title]")
        );
        assert!(find_fill_marker("no marker here").is_none());
    }

    #[test]
    fn is_placeholder_detects_common_strings() {
        assert!(is_placeholder_value("TODO"));
        assert!(is_placeholder_value("TBD"));
        assert!(is_placeholder_value("..."));
        assert!(is_placeholder_value("works well"));
        assert!(!is_placeholder_value("The system must validate inputs."));
    }
}
