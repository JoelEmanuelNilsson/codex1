from __future__ import annotations

import json
import sys
from typing import Any, TextIO


def write_json(payload: dict[str, Any], stream: TextIO | None = None) -> None:
    target = stream or sys.stdout
    target.write(json.dumps(payload, indent=2, sort_keys=True))
    target.write("\n")
