use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;

use serde_json::Value;

use crate::error::{Codex1Error, IoContext, Result};
use crate::render::{answers_from_json, AnswerValue, Answers};
use crate::template::Template;

pub fn read_answers_file(path: &Path) -> Result<Answers> {
    let text = fs::read_to_string(path)
        .io_context(format!("failed to read answers file {}", path.display()))?;
    let value: Value = serde_json::from_str(&text).map_err(|source| Codex1Error::Json {
        path: path.to_path_buf(),
        source,
    })?;
    answers_from_json(value)
}

pub fn run_interactive<R: BufRead, W: Write>(
    template: &Template,
    mut input: R,
    mut output: W,
) -> Result<Answers> {
    let mut answers = Answers::new();
    for section in template.sections {
        writeln!(
            output,
            "{}{}:",
            section.prompt,
            if section.repeatable {
                " (semicolon-separated)"
            } else {
                ""
            }
        )
        .io_context("failed to write interactive prompt")?;
        output.flush().io_context("failed to flush prompt")?;
        let mut line = String::new();
        input
            .read_line(&mut line)
            .io_context("failed to read interactive answer")?;
        let line = line.trim().to_string();
        let value = if section.repeatable {
            AnswerValue::List(
                line.split(';')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned)
                    .collect(),
            )
        } else {
            AnswerValue::Text(line)
        };
        answers.insert(section.id.to_string(), value);
    }
    Ok(answers)
}
