#!/usr/bin/env python3
"""Generate pkl/Builtins.pkl and pkl/builtins_meta.json from all builtins/*.pkl files."""

import glob
import json
import os
import subprocess
import sys
import tempfile

COMMAND_FIELDS = ("check", "check_list_files", "check_diff", "fix")

HEADER = """\
// THIS FILE IS GENERATED: Run 'mise run pkl:gen' to generate.

import "Config.pkl" as Config

"""

FACTORY_HEADER = """\

/// Indicator for detecting if a builtin is relevant to a project
class ProjectIndicator {
  /// Exact file path to check for existence
  file: String?

  /// Glob pattern to match any file
  glob: String?

  /// Content pattern to grep for (requires file to be set)
  contains: String?
}

/// Internal class for annotating hk builtins for documentation generation
class meta extends Annotation {
  /// Category for documentation grouping (e.g., "JavaScript/TypeScript", "Python", "Rust")
  category: String?

  /// Human-readable description of the step for documentation
  description: String?

  /// Project indicators for auto-detection
  project_indicators: Listing<ProjectIndicator>?
}

"""

INTERNAL_VARIANTS = {"knip_strict", "pinact_v3", "pinact_update_v3"}

OPTION_FACTORIES = {
    "gitleaks": (
        "  staged: Boolean = false\n",
        "if (staged) ({raw}.gitleaks) {{ staged = true }} else {raw}.gitleaks",
    ),
    "knip": (
        "  strict: Boolean = false\n",
        "if (strict) {knip_strict}.knip_strict else {raw}.knip",
    ),
    "pinact": (
        '  version: "3" | "4" = "4"\n',
        'if (version == "3") {pinact_v3}.pinact_v3 else {raw}.pinact',
    ),
    "pinact_update": (
        '  version: "3" | "4" = "4"\n',
        'if (version == "3") {pinact_update_v3}.pinact_update_v3 '
        'else {raw}.pinact_update',
    ),
}


def class_name(identifier):
    return "".join(part.capitalize() for part in identifier.split("_"))


def raw_alias(identifier):
    return f"Raw{class_name(identifier)}"


# Deprecated aliases. These point at a canonical builtin so loading
# Builtins.pkl never reads a deprecated property — under pklr's lazy
# @Deprecated handling (>= 0.4.2) the warning then fires only when a
# user references e.g. `Builtins.check_byte_order_marker`.
# (alias_name, canonical_name, since, message)
DEPRECATED_ALIASES = [
    (
        "check_byte_order_marker",
        "byte_order_marker",
        "1.30.0",
        "Use `Builtins.byte_order_marker`",
    ),
    (
        "fix_byte_order_marker",
        "byte_order_marker",
        "1.30.0",
        "Use `Builtins.byte_order_marker`",
    ),
]


def validate_effect_coverage():
    result = subprocess.run(
        ["pkl", "eval", "pkl/Builtins.pkl", "--format", "json"],
        capture_output=True,
        text=True,
        check=True,
    )
    builtins = json.loads(result.stdout)["all"]
    missing = []
    for name, step in builtins.items():
        if not isinstance(step, dict):
            continue
        step = step.get("step", step)
        for field in COMMAND_FIELDS:
            command = step.get(field)
            if command is not None and (
                not isinstance(command, dict)
                or command.get("effect") not in {"read", "write", "destructive"}
            ):
                missing.append(f"{name}.{field}")
    if missing:
        raise RuntimeError(
            "builtin commands missing effects:\n  " + "\n  ".join(sorted(missing))
        )


def main():
    skip = {alias for alias, _, _, _ in DEPRECATED_ALIASES} | INTERNAL_VARIANTS

    # Generate pkl/Builtins.pkl
    with open("pkl/Builtins.pkl", "w", newline="\n") as f:
        f.write(HEADER)
        for filepath in sorted(glob.glob("pkl/builtins/*.pkl")):
            filename = os.path.splitext(os.path.basename(filepath))[0]
            identifier = filename.replace("-", "_")
            f.write(f'import "builtins/{filename}.pkl" as {raw_alias(identifier)}\n')
        f.write(FACTORY_HEADER)
        for filepath in sorted(glob.glob("pkl/builtins/*.pkl")):
            filename = os.path.splitext(os.path.basename(filepath))[0]
            identifier = filename.replace("-", "_")
            if identifier in skip:
                continue
            factory_class = class_name(identifier)
            properties, expression = OPTION_FACTORIES.get(
                identifier,
                ("", f"{{raw}}.{identifier}"),
            )
            expression = expression.format(
                raw=raw_alias(identifier),
                knip_strict=raw_alias("knip_strict"),
                pinact_v3=raw_alias("pinact_v3"),
                pinact_update_v3=raw_alias("pinact_update_v3"),
            )
            f.write(f"class {factory_class} extends Config.BuiltinFactory {{\n")
            f.write(properties)
            f.write(f"  step = {expression}\n")
            f.write("}\n")
            f.write(f"{identifier} = new {factory_class} {{}}\n")

        f.write("\nall = new Mapping<String, Config.BuiltinFactory> {\n")
        for filepath in sorted(glob.glob("pkl/builtins/*.pkl")):
            filename = os.path.splitext(os.path.basename(filepath))[0]
            identifier = filename.replace("-", "_")
            if identifier in skip:
                continue
            f.write(f'  ["{identifier}"] = {identifier}\n')
        f.write("}\n")

        for alias, canonical, since, message in DEPRECATED_ALIASES:
            f.write("\n")
            f.write("@Deprecated {\n")
            f.write(f'  since = "{since}"\n')
            f.write(f'  message = "{message}"\n')
            f.write("}\n")
            f.write(f"{alias} = {canonical}\n")

    # pkl format (exits 11 after formatting, ignore that)
    subprocess.run(["pkl", "format", "--write", "pkl/Builtins.pkl"])
    validate_effect_coverage()

    # Generate builtins metadata JSON for build script
    reflect_script = os.path.join(os.getcwd(), "scripts", "reflect.pkl")
    if sys.platform == "win32":
        reflect_uri = "file:///" + reflect_script.replace("\\", "/")
    else:
        reflect_uri = "file://" + reflect_script

    entries = []
    for filepath in sorted(glob.glob("pkl/builtins/*.pkl")):
        filename = os.path.splitext(os.path.basename(filepath))[0]
        identifier = filename.replace("-", "_")
        if identifier in skip:
            continue

        # Use pkl reflection to extract metadata
        try:
            result = subprocess.run(
                [
                    "pkl",
                    "eval",
                    filepath,
                    "--format",
                    "json",
                    "-x",
                    f'import("{reflect_uri}").render(module)',
                ],
                capture_output=True,
                text=True,
                timeout=30,
            )
            if result.returncode != 0:
                continue
            raw_json = result.stdout
        except Exception:
            continue

        try:
            data = json.loads(raw_json)
            props = data.get("moduleClass", {}).get("properties", {})
            for name, prop in props.items():
                category = ""
                description = ""
                project_indicators = []

                for ann in prop.get("annotations", []):
                    if "category" in ann:
                        category = ann["category"]
                    if "description" in ann:
                        description = ann["description"]
                    if "project_indicators" in ann:
                        indicators = ann["project_indicators"]
                        if isinstance(indicators, list):
                            project_indicators = indicators

                entries.append(
                    {
                        "name": name,
                        "category": category,
                        "description": description,
                        "project_indicators": project_indicators,
                    }
                )
                break  # Only first property (the builtin definition)
        except Exception:
            continue

    # Write atomically
    fd, tmpfile = tempfile.mkstemp(
        dir="pkl", prefix="builtins_meta.json.", suffix=".tmp"
    )
    try:
        with os.fdopen(fd, "w", newline="\n") as f:
            json.dump(entries, f, indent=None)
            f.write("\n")
        os.replace(tmpfile, "pkl/builtins_meta.json")
    except Exception:
        os.unlink(tmpfile)
        raise

    print("pkl/builtins_meta.json")


if __name__ == "__main__":
    main()
