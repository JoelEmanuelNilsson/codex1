//! JSON envelope helpers — success and error envelopes shared across commands.
//!
//! Every CLI command produces a JSON object with `ok: bool` and
//! `schema: "codex1.<command>.v1"` at the top level; success adds
//! command-specific fields, error adds the fixed shape below.

// The CLI command implementations (T10-T12) will consume every helper here;
// until then call sites live in this module's unit tests.
#![allow(dead_code)]

use serde::Serialize;
use serde_json::{json, Value};

use crate::error::CliError;

/// Schema string for the error envelope.
pub const ERROR_SCHEMA: &str = "codex1.error.v1";

/// Build a success envelope as JSON. Flattens `payload`'s fields into the
/// top-level object.
///
/// The resulting JSON always has `ok: true` and `schema` set to the caller's
/// constant. Payloads that serialize to non-object values cause a runtime
/// error (we require a struct shape).
pub fn success<T: Serialize>(schema: &'static str, payload: &T) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("ok".into(), Value::Bool(true));
    obj.insert("schema".into(), Value::String(schema.into()));
    let payload_json = serde_json::to_value(payload).unwrap_or(Value::Null);
    if let Value::Object(map) = payload_json {
        for (k, v) in map {
            obj.insert(k, v);
        }
    } else {
        obj.insert("data".into(), payload_json);
    }
    Value::Object(obj)
}

/// Build an error envelope from a `CliError`.
pub fn error(err: &CliError) -> Value {
    json!({
        "ok": false,
        "schema": ERROR_SCHEMA,
        "code": err.code(),
        "message": err.to_string(),
        "retryable": err.retryable(),
        "exit_code": err.exit_code(),
        "hint": err.hint(),
        "details": err.details(),
    })
}

/// Serialize a value with stable field ordering (pretty-print disabled).
pub fn to_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

#[cfg(test)]
mod tests {
    use super::{error, success, to_string, ERROR_SCHEMA};
    use crate::error::CliError;
    use serde::Serialize;
    use serde_json::json;

    #[derive(Serialize)]
    struct InitPayload<'a> {
        mission_id: &'a str,
        state_revision: u64,
    }

    #[test]
    fn success_envelope_flattens_payload() {
        let payload = InitPayload {
            mission_id: "example",
            state_revision: 1,
        };
        let env = success("codex1.init.v1", &payload);
        assert_eq!(env["ok"], json!(true));
        assert_eq!(env["schema"], "codex1.init.v1");
        assert_eq!(env["mission_id"], "example");
        assert_eq!(env["state_revision"], 1);
    }

    #[test]
    fn error_envelope_matches_contract() {
        let err = CliError::MissionIdInvalid {
            got: "BAD/ID".into(),
        };
        let env = error(&err);
        assert_eq!(env["ok"], json!(false));
        assert_eq!(env["schema"], ERROR_SCHEMA);
        assert_eq!(env["code"], "MISSION_ID_INVALID");
        assert_eq!(env["retryable"], json!(false));
        assert_eq!(env["exit_code"], 2);
        assert!(env["hint"].is_string());
        assert_eq!(env["details"]["got"], "BAD/ID");
    }

    #[test]
    fn error_envelope_for_mission_exists_uses_exit_3() {
        let err = CliError::MissionExists {
            path: "/tmp/PLANS/example".into(),
        };
        let env = error(&err);
        assert_eq!(env["code"], "MISSION_EXISTS");
        assert_eq!(env["exit_code"], 3);
        assert_eq!(env["details"]["path"], "/tmp/PLANS/example");
    }

    #[test]
    fn to_string_is_stable_compact_json() {
        let env = success("codex1.init.v1", &json!({"mission_id": "x"}));
        let s = to_string(&env);
        assert!(s.starts_with('{'));
        assert!(s.contains("\"mission_id\":\"x\""));
    }
}
