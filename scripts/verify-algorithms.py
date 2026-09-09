#!/usr/bin/env python3
"""Independent numeric cross-check of core algorithms (see docs/ALGORITHMS.md).

Dependency-free Python reimplementation of key math. Asserts the committed
fixtures/expected/*.json values; Rust tests assert the same numbers, so a
transcription error in either implementation fails loudly (here or in CI).

Only fixtures with a registered verifier below are accepted — anything else
fails. Verifiers land with their phase (cluster/route/valuation/blend: P4–P5).
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = ROOT / "fixtures" / "expected"

PASS = 0


def check(name: str, actual: float, expected: float, tol: float = 1e-6) -> None:
    global PASS
    if not math.isclose(actual, expected, rel_tol=tol, abs_tol=tol):
        print(f"FAIL {name}: got {actual!r}, expected {expected!r}")
        sys.exit(1)
    PASS += 1
    print(f"ok  {name} = {actual!r}")


def dist(ax: float, ay: float, bx: float, by: float) -> float:
    return math.hypot(ax - bx, ay - by)


def verify_geo(fx: dict) -> None:
    """Euclidean map distance x yards_per_unit (ALGORITHMS.md §1)."""
    for case in fx["distances"]:
        got = dist(case["a"][0], case["a"][1], case["b"][0], case["b"][1])
        check(f"geo.dist[{case['id']}]", got * case["yards_per_unit"], case["yards"])


VERIFIERS = {
    "geo.json": verify_geo,
}


def main() -> int:
    files = sorted(EXPECTED.glob("*.json")) if EXPECTED.is_dir() else []
    if not files:
        print("verify-algorithms: no fixtures/expected/*.json yet (pre-Phase-4) — skip")
        return 0
    for path in files:
        fn = VERIFIERS.get(path.name)
        if fn is None:
            print(f"FAIL: fixture {path.name} has no registered verifier")
            return 1
        fn(json.loads(path.read_text(encoding="utf-8")))
    print(f"PASS: {PASS} numeric checks across {len(files)} fixture(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
