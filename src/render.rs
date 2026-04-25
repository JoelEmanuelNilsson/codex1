use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::{Codex1Error, Result};
use crate::template::Template;

#[derive(Clone, Debug)]
pub enum AnswerValue {
    Text(String),
    List(Vec<String>),
}

impl AnswerValue {
    fn is_empty(&self) -> bool {
        match self {
            Self::Text(value) => value.trim().is_empty(),
            Self::List(values) => values.iter().all(|value| value.trim().is_empty()),
        }
    }
}

pub type Answers = BTreeMap<String, AnswerValue>;

pub fn answers_from_json(value: Value) -> Result<Answers> {
    let object = value
        .as_object()
        .ok_or_else(|| Codex1Error::Interview("answers file must contain a JSON object".into()))?;
    let object = object
        .get("answers")
        .and_then(Value::as_object)
        .unwrap_or(object);

    let mut answers = Answers::new();
    for (key, value) in object {
        let answer = match value {
            Value::String(text) => AnswerValue::Text(text.clone()),
            Value::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    let text = item.as_str().ok_or_else(|| {
                        Codex1Error::Interview(format!(
                            "answer {key} must be a string or list of strings"
                        ))
                    })?;
                    out.push(text.to_string());
                }
                AnswerValue::List(out)
            }
            Value::Null => AnswerValue::Text(String::new()),
            _ => {
                return Err(Codex1Error::Interview(format!(
                    "answer {key} must be a string or list of strings"
                )))
            }
        };
        if answers.insert(key.clone(), answer).is_some() {
            return Err(Codex1Error::Interview(format!(
                "duplicate answer id: {key}"
            )));
        }
    }
    Ok(answers)
}

pub fn validate_answers(template: &Template, answers: &Answers) -> Result<()> {
    for key in answers.keys() {
        if !template.sections.iter().any(|section| section.id == key) {
            return Err(Codex1Error::Interview(format!(
                "unknown answer id for {}: {}",
                template.kind, key
            )));
        }
    }
    for section in template.sections {
        if let Some(answer) = answers.get(section.id) {
            if section.repeatable && matches!(answer, AnswerValue::Text(_)) {
                return Err(Codex1Error::Interview(format!(
                    "answer {} must be a list of strings",
                    section.id
                )));
            }
            if !section.repeatable && matches!(answer, AnswerValue::List(_)) {
                return Err(Codex1Error::Interview(format!(
                    "answer {} must be a string",
                    section.id
                )));
            }
        }
        if section.required {
            let missing = answers
                .get(section.id)
                .map(AnswerValue::is_empty)
                .unwrap_or(true);
            if missing {
                return Err(Codex1Error::Interview(format!(
                    "missing required answer: {}",
                    section.id
                )));
            }
        }
    }
    Ok(())
}

pub fn render_markdown(template: &Template, answers: &Answers) -> Result<String> {
    validate_answers(template, answers)?;
    let title = match answers.get("title") {
        Some(AnswerValue::Text(text)) if !text.trim().is_empty() => text.trim().to_string(),
        _ => template.name.to_string(),
    };
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("codex1_template: {}\n", template.kind));
    out.push_str(&format!("template_version: {}\n", template.version));
    out.push_str("---\n\n");
    out.push_str(&format!("# {title}\n\n"));
    for section in template.sections {
        if section.id == "title" {
            continue;
        }
        out.push_str(&format!("<!-- codex1-section: {} -->\n", section.id));
        out.push_str(&format!("## {}\n\n", section.heading));
        match answers.get(section.id) {
            Some(AnswerValue::Text(text)) if !text.trim().is_empty() => {
                out.push_str(text.trim());
                out.push_str("\n\n");
            }
            Some(AnswerValue::List(values)) => {
                let values: Vec<_> = values
                    .iter()
                    .filter(|value| !value.trim().is_empty())
                    .collect();
                if values.is_empty() {
                    out.push_str("_Not specified._\n\n");
                } else {
                    for value in values {
                        out.push_str("- ");
                        out.push_str(value.trim());
                        out.push('\n');
                    }
                    out.push('\n');
                }
            }
            _ => out.push_str("_Not specified._\n\n"),
        }
    }
    Ok(out)
}

pub fn render_template_outline(template: &Template) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# {} Template v{}\n\n",
        template.name, template.version
    ));
    for section in template.sections {
        out.push_str(&format!("<!-- codex1-section: {} -->\n", section.id));
        out.push_str(&format!("## {}\n\n", section.heading));
        let required = if section.required {
            "required"
        } else {
            "optional"
        };
        let repeatable = if section.repeatable {
            ", repeatable"
        } else {
            ""
        };
        out.push_str(&format!("_{required}{repeatable}. {}_\n\n", section.prompt));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ArtifactKind;
    use crate::template;

    #[test]
    fn renders_prd_with_section_tags() {
        let template = template::get(ArtifactKind::Prd);
        let answers = answers_from_json(serde_json::json!({
            "title": "Example",
            "original_request": "Build it",
            "interpreted_destination": "A useful thing",
            "success_criteria": ["works"],
            "proof_expectations": ["tests"],
            "pr_intent": "No PR"
        }))
        .unwrap();
        let rendered = render_markdown(&template, &answers).unwrap();
        assert!(rendered.contains("# Example"));
        assert!(rendered.contains("<!-- codex1-section: success_criteria -->"));
        assert!(rendered.contains("- works"));
    }

    #[test]
    fn missing_required_answer_fails() {
        let template = template::get(ArtifactKind::Plan);
        let answers = Answers::new();
        assert!(render_markdown(&template, &answers).is_err());
    }
}
