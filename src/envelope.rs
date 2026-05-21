use serde::Serialize;
use serde_json::json;

use crate::error::Codex1Error;

#[derive(Debug, Serialize)]
pub struct SuccessEnvelope<T: Serialize> {
    pub ok: bool,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope<'a> {
    pub ok: bool,
    pub error: ErrorBody<'a>,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody<'a> {
    pub code: &'a str,
    pub message: String,
}

pub fn success<T: Serialize>(data: T) -> serde_json::Value {
    serde_json::to_value(SuccessEnvelope { ok: true, data }).unwrap_or_else(|_| {
        json!({
            "ok": true,
            "data": null
        })
    })
}

pub fn error(error: &Codex1Error) -> serde_json::Value {
    serde_json::to_value(ErrorEnvelope {
        ok: false,
        error: ErrorBody {
            code: error.code().as_str(),
            message: error.to_string(),
        },
    })
    .unwrap_or_else(|_| {
        json!({
            "ok": false,
            "error": {
                "code": "IO_ERROR",
                "message": "failed to serialize error envelope"
            }
        })
    })
}
