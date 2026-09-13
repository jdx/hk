#!/usr/bin/env python3
"""Validate shipped and Bats-authored v2 configurations with Apple Pkl."""

from __future__ import annotations

import re
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
PKL_DIR = (ROOT / "pkl").resolve()
PACKAGE_ROOT = re.compile(
    r"package://github\.com/jdx/hk/releases/download/v[^/]+/hk@[^#]+#/"
)
REWRITTEN_PACKAGE_ROOT = re.compile(r"package://example\.com/v[^/]+/hk@[^#]+#/")
HEREDOC = re.compile(r"<<-?\s*(['\"]?)([A-Za-z0-9_]+)\1")
REDIRECT = re.compile(r"(>>|>)\s*(['\"]?)([^\s'\"]*\.pkl)\2")
TEST = re.compile(r'^\s*@test\s+["\']([^"\']+)["\']\s*\{')
FUNCTION = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\(\)\s*\{")
SHELL_EXPANSIONS = {
    "$first_effect": "read",
    "$second_effect": "write",
    "$oversized": "x",
    "$stash_method": "git",
    "$method": "git",
    "$(pwd)": "/tmp/hk-apple-pkl",
    "${version}": "1.58.1",
    "$HK_REPORT_JSON": r"\$HK_REPORT_JSON",
    "$NORMAL_INDEX": "/tmp/hk-apple-pkl/index",
    "$WORKTREE_DIR": "/tmp/hk-apple-pkl/worktree",
}

# These fixtures deliberately exercise Pkl evaluation errors. The migration
# fixtures import removed v1 shims; the others verify malformed-config errors.
EXPECTED_FAILURES = {
    ("config_error_handling.bats", "hk check fails on invalid config"),
    ("config_error_handling.bats", "hk fix fails on invalid config"),
    ("config_error_handling.bats", "hk run fails on invalid config"),
    ("config_error_handling.bats", "hk config commands fail on invalid config"),
    ("config_error_handling.bats", "hk util ignores invalid project and user config"),
    ("config_error_handling.bats", "config error shows helpful details"),
    ("pkl_config_errors.bats", "missing amends declaration shows helpful error"),
    ("pkl_config_errors.bats", "invalid module URI shows helpful error"),
    ("pkl_config_errors.bats", "pkl file with syntax errors shows original error"),
    ("regex_patterns.bats", "Config.Regex fails with v2 migration guidance"),
    ("regex_patterns.bats", "Types.Regex fails with v2 migration guidance"),
    (
        "v2_migration_errors.bats",
        "removed byte-order-marker aliases have migration guidance",
    ),
    (
        "v2_migration_errors.bats",
        "removed fix byte-order-marker alias has migration guidance",
    ),
}


@dataclass(frozen=True)
class Heredoc:
    line: int
    scope: str
    target: str
    append: bool
    source: str


@dataclass(frozen=True)
class Fixture:
    name: str
    target: str
    source: str
    expected_failure: bool
    support: tuple[tuple[str, str], ...]


def render(source: str) -> str:
    source = source.replace("$PKL_PATH", PKL_DIR.as_posix())
    for expression, value in SHELL_EXPANSIONS.items():
        source = re.sub(rf"(?<!\\){re.escape(expression)}", lambda _: value, source)
    source = source.replace(r"\$", "$")
    source = PACKAGE_ROOT.sub(f"{PKL_DIR.as_posix()}/", source)
    return REWRITTEN_PACKAGE_ROOT.sub(f"{PKL_DIR.as_posix()}/", source)


def heredocs(path: Path) -> list[Heredoc]:
    lines = path.read_text().splitlines()
    blocks: list[Heredoc] = []
    scope = "file"
    index = 0
    while index < len(lines):
        line = lines[index]
        test_match = TEST.match(line)
        function_match = FUNCTION.match(line)
        if test_match:
            scope = test_match.group(1)
        elif function_match:
            scope = function_match.group(1)

        marker_match = HEREDOC.search(line)
        redirect_match = REDIRECT.search(line)
        if "cat" not in line or marker_match is None or redirect_match is None:
            index += 1
            continue

        marker = marker_match.group(2)
        body: list[str] = []
        start_line = index + 1
        index += 1
        while index < len(lines) and lines[index].strip() != marker:
            body.append(lines[index])
            index += 1
        if index == len(lines):
            raise RuntimeError(f"unterminated heredoc in {path}:{start_line}")
        blocks.append(
            Heredoc(
                line=start_line,
                scope=scope,
                target=redirect_match.group(3),
                append=redirect_match.group(1) == ">>",
                source="\n".join(body) + "\n",
            )
        )
        index += 1
    return blocks


def bats_fixtures() -> list[Fixture]:
    fixtures: list[Fixture] = []
    found_expected_failures: set[tuple[str, str]] = set()
    for path in sorted((ROOT / "test").glob("*.bats")):
        blocks = heredocs(path)
        support_by_scope: dict[str, list[tuple[str, str]]] = {}
        for block in blocks:
            if Path(block.target).name != "hk.pkl":
                support_by_scope.setdefault(block.scope, []).append(
                    (block.target, block.source)
                )
        latest: dict[str, str] = {}
        for block in blocks:
            if Path(block.target).name != "hk.pkl":
                continue
            source = block.source
            if block.append:
                try:
                    source = latest[block.target] + source
                except KeyError as error:
                    raise RuntimeError(
                        f"append without a preceding {block.target} fixture "
                        f"in {path}:{block.line}"
                    ) from error
            else:
                latest[block.target] = source

            failure_key = (path.name, block.scope)
            expected_failure = failure_key in EXPECTED_FAILURES
            if expected_failure:
                found_expected_failures.add(failure_key)
            fixtures.append(
                Fixture(
                    name=f"{path.stem}-{block.line}.pkl",
                    target=block.target,
                    source=source,
                    expected_failure=expected_failure,
                    support=tuple(support_by_scope.get(block.scope, [])),
                )
            )

    missing = EXPECTED_FAILURES - found_expected_failures
    if missing:
        raise RuntimeError(f"expected failing fixtures were not found: {sorted(missing)}")
    return fixtures


def sandbox_path(root: Path, target: str) -> Path:
    relative = re.sub(r"^\$([A-Za-z_][A-Za-z0-9_]*)/", r"\1/", target)
    path = Path(relative)
    if path.is_absolute() or ".." in path.parts:
        raise RuntimeError(f"unsafe fixture target: {target}")
    return root / path


def evaluate(
    path: Path, *, expected_failure: bool = False, label: str | Path | None = None
) -> None:
    result = subprocess.run(
        ["pkl", "eval", "--format", "json", str(path)],
        cwd=ROOT,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    label = label or (path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name)
    if expected_failure:
        if result.returncode == 0:
            raise RuntimeError(f"{label} unexpectedly evaluated successfully")
        print(f"{label} (expected failure)")
    elif result.returncode != 0:
        raise RuntimeError(f"{label} failed Apple Pkl evaluation:\n{result.stderr}")
    else:
        print(label)


def main() -> None:
    evaluate(ROOT / "hk.pkl")
    with tempfile.TemporaryDirectory(prefix="hk-apple-pkl-") as temp:
        temp_dir = Path(temp)
        for source in sorted((ROOT / "docs/public").glob("*.pkl")):
            rendered = temp_dir / f"docs-{source.name}"
            rendered.write_text(render(source.read_text()))
            evaluate(rendered)
        for fixture in bats_fixtures():
            fixture_dir = temp_dir / fixture.name.removesuffix(".pkl")
            fixture_dir.mkdir()
            for target, source in fixture.support:
                support = sandbox_path(fixture_dir, target)
                support.parent.mkdir(parents=True, exist_ok=True)
                support.write_text(render(source))
            rendered = sandbox_path(fixture_dir, fixture.target)
            rendered.parent.mkdir(parents=True, exist_ok=True)
            rendered.write_text(render(fixture.source))
            evaluate(
                rendered,
                expected_failure=fixture.expected_failure,
                label=fixture.name,
            )


if __name__ == "__main__":
    main()
