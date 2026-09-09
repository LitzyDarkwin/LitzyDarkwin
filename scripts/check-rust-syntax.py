#!/usr/bin/env python3
"""Rust syntax gate for environments without a Rust toolchain.

Parses every crates/**.rs and src-tauri/**.rs file with tree-sitter-rust and
fails on ERROR/MISSING nodes. This catches syntax slips, not type errors;
full `cargo check/test/clippy` still runs in CI.

Skips gracefully (exit 0) when tree-sitter-rust is not installed.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGETS = [
    ROOT / "crates",
    ROOT / "apps" / "desktop" / "src-tauri" / "src",
    ROOT / "apps" / "desktop" / "src-tauri" / "build.rs",
]


def has_error(node) -> bool:
    if node.type == "ERROR" or node.is_missing:
        return True
    return any(has_error(child) for child in node.children)


def first_error(node, source: bytes):
    if node.type == "ERROR" or node.is_missing:
        line = source[: node.start_byte].count(b"\n") + 1
        snippet = source[node.start_byte: node.start_byte + 80].decode("utf8", "replace")
        return line, snippet
    for child in node.children:
        found = first_error(child, source)
        if found:
            return found
    return None


def main() -> int:
    try:
        from tree_sitter import Language, Parser
        import tree_sitter_rust
    except ImportError:
        print("check-rust-syntax: tree-sitter-rust not installed — skip")
        return 0

    parser = Parser(Language(tree_sitter_rust.language()))
    files = []
    for target in TARGETS:
        if target.is_file():
            files.append(target)
        elif target.is_dir():
            files.extend(sorted(target.rglob("*.rs")))
    failed = 0
    for path in files:
        source = path.read_bytes()
        tree = parser.parse(source)
        if has_error(tree.root_node):
            failed += 1
            err = first_error(tree.root_node, source)
            print(f"FAIL  {path.relative_to(ROOT)}:{err[0]}: {err[1]!r}")
        else:
            print(f"ok  {path.relative_to(ROOT)}")
    if failed:
        print(f"check-rust-syntax: {failed} file(s) with syntax errors")
        return 1
    print(f"check-rust-syntax: PASS ({len(files)} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
