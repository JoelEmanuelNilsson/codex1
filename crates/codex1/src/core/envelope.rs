//! Stable JSON envelopes shared by every command.
//!
//! Success:
//! ```json
//! { "ok": true, "mission_id": "demo", "revision": 7, "data": { ... } }
//! ```
//!
//! Error:
//! ```json
//! { "ok": false, "code": "PLAN_INVALID", "message": "…", "hint": "…",
//!   "retryable": false, "context": {} }
//! ```
//!
//! Downstream units serialize their command-specific data into the `data`
//! field of `JsonOk`. Errors use the `CliError` types from `core::error`.

use serde::Serialize;
use serde_json::Value;

/// Success envelope.
#[derive(Debug, Clone, Serialize)]
pub struct JsonOk {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    pub data: Value,
}

impl JsonOk {
    /// Build a success envelope with command-specific data.
    #[must_use]
    pub fn new(mission_id: Option<String>, revision: Option<u64>, data: Value) -> Self {
        Self {
            ok: true,
            mission_id,
            revision,
            data,
        }
    }

    /// Build a success envelope with no mission binding (e.g. doctor).
    #[must_use]
    pub fn global(data: Value) -> Self {
        Self {
            ok: true,
            mission_id: None,
            revision: None,
            data,
        }
    }

    /// Serialize the envelope to a pretty JSON string.
    #[must_use]
    pub fn to_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{\"ok\":true}".to_string())
    }

    /// Serialize the envelope to a compact JSON string (for machine pipes).
    #[must_use]
    pub fn to_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"ok\":true}".to_string())
    }
}

/// Error envelope. Stable across every command.
#[derive(Debug, Clone, Serialize)]
pub struct JsonErr {
    pub ok: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub context: Value,
}

impl JsonErr {
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        hint: Option<String>,
        retryable: bool,
        context: Value,
    ) -> Self {
        Self {
            ok: false,
            code: code.into(),
            message: message.into(),
            hint,
            retryable,
            context,
        }
    }

    #[must_use]
    pub fn to_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    }

    #[must_use]
    pub fn to_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    }
}
