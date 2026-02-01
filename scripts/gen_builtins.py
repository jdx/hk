#!/usr/bin/env python3
"""Generate pkl/Builtins.pkl and pkl/builtins_meta.json from all builtins/*.pkl files."""

import glob
import json
import os
import subprocess
import sys
import tempfile

HEADER = """\
// THIS FILE IS GENERATED: Run 'mise run pkl:gen' to generate.

import* "builtins/*.pkl" as Builtins

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


def main():
    # Generate pkl/Builtins.pkl
    with open("pkl/Builtins.pkl", "w", newline="\n") as f:
        f.write(HEADER)
        for filepath in sorted(glob.glob("pkl/builtins/*.pkl")):
            filename = os.path.splitext(os.path.basename(filepath))[0]
            identifier = filename.replace("-", "_")
            f.write(f'{identifier} = Builtins["builtins/{filename}.pkl"].{identifier}\n')

    # pkl format (exits 11 after formatting, ignore that)
    subprocess.run(["pkl", "format", "--write", "pkl/Builtins.pkl"])

    # Generate builtins metadata JSON for build script
    reflect_script = os.path.join(os.getcwd(), "scripts", "reflect.pkl")
    if sys.platform == "win32":
        reflect_uri = "file:///" + reflect_script.replace("\\", "/")
    else:
        reflect_uri = "file://" + reflect_script

    entries = []
    for filepath in sorted(glob.glob("pkl/builtins/*.pkl")):
        filename = os.path.splitext(os.path.basename(filepath))[0]

        # Use pkl reflection to extract metadata
        try:
            result = subprocess.run(
                ["pkl", "eval", filepath, "--format", "json", "-x",
                 f'import("{reflect_uri}").render(module)'],
                capture_output=True, text=True, timeout=30,
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

                entries.append({
                    "name": name,
                    "category": category,
                    "description": description,
                    "project_indicators": project_indicators,
                })
                break  # Only first property (the builtin definition)
        except Exception:
            continue

    # Write atomically
    fd, tmpfile = tempfile.mkstemp(dir="pkl", prefix="builtins_meta.json.", suffix=".tmp")
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
