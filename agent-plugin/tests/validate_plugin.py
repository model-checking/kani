#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Validate the Kani agent plugin's metadata and skill structure.

Checks that the manifests parse and have the required fields, that every file the manifests
and the skill point to exists, that versions agree, and that SKILL.md has valid frontmatter.
Runs without Kani or a Rust toolchain.
"""

import json
import re
import sys
from pathlib import Path

PLUGIN = Path(__file__).resolve().parent.parent
REQUIRED_PLUGIN_FIELDS = ["name", "version", "description", "author", "keywords", "license", "repository"]
REQUIRED_REFERENCES = ["kani", "autoharness", "contracts", "miri", "rust-analyzer", "strategy", "vacuity"]
REQUIRED_SCRIPTS = [
    "check-assumptions.py", "check-kani-version.py", "check-proof-diff.py", "check-specs.py",
    "check-vacuity.py", "check-witness.py", "evidence.py", "inspect-harness.py", "summarize-results.py",
]
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")
SKILL_NAME = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
MAX_DESCRIPTION = 1024

errors = []


def check(cond, msg):
    if not cond:
        errors.append(msg)


def load_json(name):
    try:
        return json.loads((PLUGIN / name).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        errors.append(f"{name}: {e}")
        return {}


def frontmatter(path):
    text = path.read_text(encoding="utf-8")
    m = re.match(r"^---\n(.*?)\n---\n", text, re.S)
    if not m:
        return None, text
    fields = {}
    for line in m.group(1).splitlines():
        key, sep, value = line.partition(":")
        if sep:
            fields[key.strip()] = value.strip()
    return fields, text[m.end():]


def main():
    plugin = load_json("plugin.json")
    for field in REQUIRED_PLUGIN_FIELDS:
        check(plugin.get(field), f"plugin.json: missing `{field}`")
    check(SEMVER.match(str(plugin.get("version", ""))), "plugin.json: `version` is not semver")
    for key in ("skills", "mcpServers", "logo"):
        if key in plugin:
            check((PLUGIN / plugin[key]).exists(), f"plugin.json: `{key}` points to a missing path")

    mcp = load_json("mcp.json")
    servers = mcp.get("mcpServers", {})
    check("kani" in servers, "mcp.json: missing the `kani` server")
    for name, server in servers.items():
        check(server.get("command"), f"mcp.json: server `{name}` has no command")
        check("url" not in server, f"mcp.json: server `{name}` must be a local stdio server")

    compat = load_json("compatibility.json")
    check(compat.get("plugin_version") == plugin.get("version"),
          "compatibility.json: `plugin_version` does not match plugin.json `version`")
    for key in ("min_kani", "max_kani_exclusive"):
        check(SEMVER.match(str(compat.get(key, ""))), f"compatibility.json: `{key}` is not semver")
    if all(SEMVER.match(str(compat.get(k, ""))) for k in ("min_kani", "max_kani_exclusive")):
        lo, hi = (tuple(map(int, compat[k].split("."))) for k in ("min_kani", "max_kani_exclusive"))
        check(lo < hi, "compatibility.json: empty supported range")

    cargo = (PLUGIN.parent / "tools" / "kani-mcp" / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.M)
    check(m and m.group(1) == plugin.get("version"),
          "tools/kani-mcp/Cargo.toml: version does not match plugin.json `version`")

    skills = sorted((PLUGIN / "skills").glob("*/SKILL.md"))
    check(skills, "no skills found under skills/")
    for skill in skills:
        rel = skill.relative_to(PLUGIN)
        fields, body = frontmatter(skill)
        if fields is None:
            errors.append(f"{rel}: missing YAML frontmatter")
            continue
        check(fields.get("name") == skill.parent.name, f"{rel}: `name` must match its directory")
        check(SKILL_NAME.match(fields.get("name", "")), f"{rel}: invalid `name`")
        description = fields.get("description", "")
        check(description, f"{rel}: missing `description`")
        check(len(description) <= MAX_DESCRIPTION, f"{rel}: `description` is too long")
        for ref in re.findall(r"`(references/[^`]+\.md)`", body):
            check((skill.parent / ref).exists(), f"{rel}: links to missing {ref}")
        for script in re.findall(r"`scripts/([A-Za-z0-9_-]+\.py)", body):
            check((PLUGIN / "scripts" / script).exists(), f"{rel}: mentions missing scripts/{script}")

    verify = PLUGIN / "skills" / "verify-rust"
    for ref in REQUIRED_REFERENCES:
        check((verify / "references" / f"{ref}.md").exists(), f"missing references/{ref}.md")
    for script in REQUIRED_SCRIPTS:
        path = PLUGIN / "scripts" / script
        check(path.exists(), f"missing scripts/{script}")
        if path.exists():
            check(path.stat().st_mode & 0o111, f"scripts/{script} is not executable")
            check(path.read_text(encoding="utf-8").startswith("#!/usr/bin/env python3"),
                  f"scripts/{script} has no python3 shebang")

    if errors:
        for e in errors:
            print(f"error: {e}")
        return 1
    print(f"plugin `{plugin['name']}` {plugin['version']}: metadata and skill structure are valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
