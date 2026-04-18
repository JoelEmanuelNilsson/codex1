//! Marker constants for the `PROGRAM-BLUEPRINT.md` YAML block.
//!
//! The plan DAG is stored in a Markdown file so humans can read the
//! surrounding prose. The CLI only reads the YAML between these two HTML
//! comments; everything else is free-form narrative.

// T11/T12 will be non-test callers.
#![allow(dead_code)]

use std::path::Path;

use crate::error::CliError;

pub const START_MARKER: &str = "<!-- codex1:plan-dag:start -->";
pub const END_MARKER: &str = "<!-- codex1:plan-dag:end -->";

/// Extract the YAML body between the markers. Returns `DagNoBlock` if either
/// marker is absent or the end appears before the start.
pub fn extract_block<'a>(path: &Path, content: &'a str) -> Result<&'a str, CliError> {
    let start = content
        .find(START_MARKER)
        .ok_or_else(|| CliError::DagNoBlock {
            path: path.display().to_string(),
        })?;
    let after_start = start + START_MARKER.len();
    let end_rel = content[after_start..]
        .find(END_MARKER)
        .ok_or_else(|| CliError::DagNoBlock {
            path: path.display().to_string(),
        })?;
    let end = after_start + end_rel;
    Ok(&content[after_start..end])
}

#[cfg(test)]
mod tests {
    use super::{extract_block, END_MARKER, START_MARKER};
    use std::path::Path;

    fn doc(body: &str) -> String {
        format!("# prose\n\n{START_MARKER}\n{body}\n{END_MARKER}\n")
    }

    #[test]
    fn extracts_yaml_between_markers() {
        let d = doc("tasks: []");
        let yaml = extract_block(Path::new("/x.md"), &d).unwrap();
        assert!(yaml.contains("tasks: []"));
    }

    #[test]
    fn missing_start_marker_errors() {
        let d = format!("prose\n{END_MARKER}\n");
        let err = extract_block(Path::new("/x.md"), &d).unwrap_err();
        assert_eq!(err.code(), "DAG_NO_BLOCK");
    }

    #[test]
    fn missing_end_marker_errors() {
        let d = format!("prose\n{START_MARKER}\ntasks: []\n");
        let err = extract_block(Path::new("/x.md"), &d).unwrap_err();
        assert_eq!(err.code(), "DAG_NO_BLOCK");
    }

    #[test]
    fn end_before_start_is_missing_start() {
        let d = format!("{END_MARKER}\n...\n{START_MARKER}\n");
        // find(START) succeeds at some position; find(END) in the slice after
        // START may not be present → DagNoBlock.
        let err = extract_block(Path::new("/x.md"), &d).unwrap_err();
        assert_eq!(err.code(), "DAG_NO_BLOCK");
    }

    #[test]
    fn multiline_yaml_is_preserved() {
        let body = "planning:\n  requested_level: light\ntasks: []";
        let d = doc(body);
        let yaml = extract_block(Path::new("/x.md"), &d).unwrap();
        assert!(yaml.contains("requested_level: light"));
        assert!(yaml.contains("tasks: []"));
    }
}
