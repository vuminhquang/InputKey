from __future__ import annotations

from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
EXCLUDED_TOP = {"target", "dist", ".git"}
FORBIDDEN_DIRS = {"shared", "common", "utils", "helpers"}
errors: list[str] = []


def rel(path: Path) -> Path:
    return path.relative_to(ROOT)


def excluded(path: Path) -> bool:
    try:
        parts = rel(path).parts
    except ValueError:
        return True
    return bool(parts and parts[0] in EXCLUDED_TOP)


def cargo_dependency_names(doc: dict) -> set[str]:
    names: set[str] = set()
    for key in ("dependencies", "dev-dependencies", "build-dependencies"):
        table = doc.get(key, {})
        if isinstance(table, dict):
            names.update(table.keys())
    target_table = doc.get("target", {})
    if isinstance(target_table, dict):
        for cfg in target_table.values():
            if not isinstance(cfg, dict):
                continue
            for key in ("dependencies", "dev-dependencies", "build-dependencies"):
                table = cfg.get(key, {})
                if isinstance(table, dict):
                    names.update(table.keys())
    return names


# Structural checks.
for p in ROOT.rglob("*"):
    if excluded(p):
        continue
    if p.is_dir() and p.name.lower() in FORBIDDEN_DIRS:
        errors.append(f"forbidden catch-all directory: {rel(p)}")
    if p.is_file() and p.suffix.lower() == ".go":
        errors.append(f"production Go file: {rel(p)}")

# Workspace and Cargo dependency direction.
workspace_doc = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
members = workspace_doc.get("workspace", {}).get("members", [])
if not isinstance(members, list):
    errors.append("workspace.members must be a list")
    members = []

for member in members:
    member_path = ROOT / str(member)
    if not member_path.exists():
        errors.append(f"workspace member does not exist: {member}")
        continue
    parts = Path(str(member)).parts
    if not parts or parts[0] not in {
        "Boundary", "CoreAbstractions", "Operators", "Implementations", "Bootstrap"
    }:
        errors.append(f"workspace member is outside declared architecture roles: {member}")
    if any(part.lower() in FORBIDDEN_DIRS for part in parts):
        errors.append(f"workspace member uses forbidden dump directory: {member}")

for manifest in ROOT.rglob("Cargo.toml"):
    if excluded(manifest) or manifest == ROOT / "Cargo.toml":
        continue
    doc = tomllib.loads(manifest.read_text(encoding="utf-8"))
    package = doc.get("package", {})
    name = package.get("name", str(rel(manifest.parent)))
    role = rel(manifest).parts[0]
    deps = cargo_dependency_names(doc)

    if role in {"Boundary", "CoreAbstractions"} and deps:
        errors.append(f"{name}: {role} must have zero Cargo dependencies: {sorted(deps)}")

    if role == "Operators":
        allowed = {"inputkey-boundary", "inputkey-core-abstractions"}
        illegal = deps - allowed
        if illegal:
            errors.append(f"{name}: Operators dependency violation: {sorted(illegal)}")

    if role == "Implementations":
        illegal_internal = {
            dep for dep in deps
            if dep == "inputkey-operators"
            or dep.startswith("inputkey-bootstrap")
            or dep in {"inputkey-cabi", "inputkey-wasm", "inputkey-windows-app"}
        }
        if illegal_internal:
            errors.append(
                f"{name}: Implementation depends upward on Operators/Bootstrap: "
                f"{sorted(illegal_internal)}"
            )

# Purity checks for semantic core Rust.
TECH_PATTERNS = [
    r"\bwindows_sys\b",
    r"\bwindows::",
    r"\bstd::fs\b",
    r"\bstd::net\b",
    r"\bstd::process\b",
    r"\bextern\s+\"C\"",
    r"\bwasm_bindgen\b",
    r"\bweb_sys\b",
    r"\btokio\b",
    r"\bserde\b",
    r"\bobjc\b",
    r"\bcocoa\b",
    r"\bibus\b",
    r"\bfcitx\b",
]
for role in ("Boundary", "CoreAbstractions", "Operators"):
    base = ROOT / role
    if not base.exists():
        continue
    for source in base.rglob("*.rs"):
        text = source.read_text(encoding="utf-8")
        for pattern in TECH_PATTERNS:
            if re.search(pattern, text, re.IGNORECASE):
                errors.append(f"technology token in {role}: {rel(source)} matches {pattern}")
        if role == "Operators" and re.search(r"\bunsafe\b", text):
            errors.append(f"unsafe code is forbidden in Operators: {rel(source)}")

if errors:
    print("CONTRACT IS LAW! ARCHITECTURE VIOLATION DETECTED.")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("Architecture checks passed: dependency direction is intact.")
