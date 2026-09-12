#!/usr/bin/env python3
"""Validate shipped and Bats-authored v2 configurations with Apple Pkl."""

from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
PKL_DIR = (ROOT / "pkl").resolve()
PACKAGE_ROOT = re.compile(
    r"package://github\.com/jdx/hk/releases/download/v[^/]+/hk@[^#]+#/"
)


def render(source: str) -> str:
    source = source.replace("$PKL_PATH", PKL_DIR.as_posix())
    source = source.replace(r"\$", "$")
    return PACKAGE_ROOT.sub(f"{PKL_DIR.as_posix()}/", source)


def heredocs(path: Path) -> list[tuple[bool, str]]:
    lines = path.read_text().splitlines()
    blocks: list[tuple[bool, str]] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        if "cat" not in line or "hk.pkl" not in line or "<<" not in line:
            index += 1
            continue
        marker_match = re.search(r"<<'?([A-Za-z0-9_]+)'?", line)
        if marker_match is None:
            raise RuntimeError(f"cannot parse heredoc marker in {path}:{index + 1}")
        marker = marker_match.group(1)
        append = re.search(r">>\s*hk\.pkl", line) is not None
        body: list[str] = []
        index += 1
        while index < len(lines) and lines[index].strip() != marker:
            body.append(lines[index])
            index += 1
        if index == len(lines):
            raise RuntimeError(f"unterminated heredoc in {path}")
        blocks.append((append, "\n".join(body) + "\n"))
        index += 1
    return blocks


def bats_fixtures() -> list[tuple[str, str]]:
    builtins = heredocs(ROOT / "test/builtins_tests.bats")
    if len(builtins) != 8 or any(append for append, _ in builtins):
        raise RuntimeError("unexpected builtins_tests.bats hk.pkl fixture layout")

    top_level = heredocs(ROOT / "test/top_level_steps.bats")
    if len(top_level) != 4 or [append for append, _ in top_level] != [False, True, True, False]:
        raise RuntimeError("unexpected top_level_steps.bats hk.pkl fixture layout")
    base = top_level[0][1]

    fixtures = [
        (f"builtins-{index}.pkl", body)
        for index, (_, body) in enumerate(builtins, start=1)
    ]
    fixtures.extend(
        [
            ("top-level-base.pkl", base),
            ("top-level-explicit.pkl", base + top_level[1][1]),
            ("top-level-disabled.pkl", base + top_level[2][1]),
            ("top-level-all-builtins.pkl", top_level[3][1]),
        ]
    )
    return fixtures


def evaluate(path: Path) -> None:
    subprocess.run(
        ["pkl", "eval", "--format", "json", str(path)],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    print(path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name)


def main() -> None:
    evaluate(ROOT / "hk.pkl")
    with tempfile.TemporaryDirectory(prefix="hk-apple-pkl-") as temp:
        temp_dir = Path(temp)
        for source in sorted((ROOT / "docs/public").glob("*.pkl")):
            rendered = temp_dir / f"docs-{source.name}"
            rendered.write_text(render(source.read_text()))
            evaluate(rendered)
        for name, source in bats_fixtures():
            rendered = temp_dir / name
            rendered.write_text(render(source))
            evaluate(rendered)


if __name__ == "__main__":
    main()
