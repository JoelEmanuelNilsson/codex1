use std::fmt;
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::{Map, Number, Value};

use crate::error::{Codex1Error, IoContext, Result};
use crate::render::{answers_from_json, AnswerValue, Answers};
use crate::template::Template;

pub fn read_answers_file(path: &Path) -> Result<Answers> {
    let text = fs::read_to_string(path)
        .io_context(format!("failed to read answers file {}", path.display()))?;
    let value: NoDuplicateValue =
        serde_json::from_str(&text).map_err(|source| Codex1Error::Json {
            path: path.to_path_buf(),
            source,
        })?;
    answers_from_json(value.0)
}

struct NoDuplicateValue(Value);

impl<'de> Deserialize<'de> for NoDuplicateValue {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateVisitor)
    }
}

struct NoDuplicateVisitor;

impl<'de> Visitor<'de> for NoDuplicateVisitor {
    type Value = NoDuplicateValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("valid JSON without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> std::result::Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        let number = Number::from_f64(value)
            .ok_or_else(|| E::custom("floating point value cannot be represented as JSON"))?;
        Ok(NoDuplicateValue(Value::Number(number)))
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::String(value.to_string())))
    }

    fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Null))
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        NoDuplicateValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<NoDuplicateValue>()? {
            values.push(value.0);
        }
        Ok(NoDuplicateValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
            }
            let value = map.next_value::<NoDuplicateValue>()?;
            object.insert(key, value.0);
        }
        Ok(NoDuplicateValue(Value::Object(object)))
    }
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
