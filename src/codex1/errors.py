from __future__ import annotations

from dataclasses import dataclass
from typing import Any


ERROR_SCHEMA_VERSION = "codex1.error.v1"


@dataclass(slots=True)
class Codex1Error(Exception):
    code: str
    message: str
    exit_code: int = 1
    retryable: bool = False
    details: dict[str, Any] | None = None

    def to_json(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "ok": False,
            "schema_version": ERROR_SCHEMA_VERSION,
            "code": self.code,
            "message": self.message,
            "retryable": self.retryable,
        }
        if self.details:
            payload.update(self.details)
        return payload
