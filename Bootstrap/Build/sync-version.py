from __future__ import annotations

import argparse
import json
import plistlib
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VERSION_FILE = ROOT / "VERSION"
CARGO_TOML = ROOT / "Cargo.toml"
CARGO_LOCK = ROOT / "Cargo.lock"
MANIFEST = ROOT / "Implementations" / "Chrome" / "extension" / "manifest.json"
FCITX = ROOT / "Implementations" / "Linux" / "Fcitx5" / "CMakeLists.txt"
IBUS = ROOT / "Implementations" / "Linux" / "IBus" / "CMakeLists.txt"
PLIST = ROOT / "Implementations" / "macOS" / "InputMethodKit" / "Info.plist"


def canonical_version() -> str:
    version = VERSION_FILE.read_text(encoding="utf-8-sig").strip()
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError(f"VERSION must be x.y.z, got {version!r}")
    return version


def replace_one(path: Path, pattern: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise RuntimeError(f"{path.relative_to(ROOT)}: expected one match for {pattern!r}, got {count}")
    path.write_text(updated, encoding="utf-8", newline="")


def sync_lock(version: str) -> None:
    text = CARGO_LOCK.read_text(encoding="utf-8")
    blocks = text.split("[[package]]")
    changed = [blocks[0]]
    for block in blocks[1:]:
        name = re.search(r'^\s*name\s*=\s*"([^"]+)"', block, re.MULTILINE)
        if name and (name.group(1) == "inputkey" or name.group(1).startswith("inputkey-")):
            block, count = re.subn(
                r'^(\s*version\s*=\s*)"[^"]+"',
                rf'\g<1>"{version}"',
                block,
                count=1,
                flags=re.MULTILINE,
            )
            if count != 1:
                raise RuntimeError(f"Cargo.lock: missing version for {name.group(1)}")
        changed.append(block)
    CARGO_LOCK.write_text("[[package]]".join(changed), encoding="utf-8", newline="")


def write(version: str) -> None:
    replace_one(
        CARGO_TOML,
        r'^(version\s*=\s*)"[^"]+"\s*$',
        rf'\g<1>"{version}"',
    )
    sync_lock(version)

    replace_one(
        MANIFEST,
        r'^(\s*"version"\s*:\s*)"[^"]+"(,\s*)$',
        rf'\g<1>"{version}"\g<2>',
    )
    replace_one(
        MANIFEST,
        r'^(\s*"version_name"\s*:\s*)"[^"]+"(,\s*)$',
        rf'\g<1>"{version}"\g<2>',
    )

    replace_one(
        FCITX,
        r"^(project\(inputkey-fcitx5 VERSION )\d+\.\d+\.\d+( LANGUAGES C CXX\))$",
        rf"\g<1>{version}\g<2>",
    )
    replace_one(
        IBUS,
        r"^(project\(inputkey-ibus VERSION )\d+\.\d+\.\d+( LANGUAGES C\))$",
        rf"\g<1>{version}\g<2>",
    )

    text = PLIST.read_text(encoding="utf-8")
    for key in ("CFBundleShortVersionString", "CFBundleVersion"):
        pattern = rf'(<key>{re.escape(key)}</key>\s*<string>)[^<]+(</string>)'
        text, count = re.subn(pattern, rf'\g<1>{version}\g<2>', text, count=1)
        if count != 1:
            raise RuntimeError(f"{PLIST.relative_to(ROOT)}: expected one {key} value")
    PLIST.write_text(text, encoding="utf-8", newline="")


def collect(version: str) -> list[str]:
    errors: list[str] = []

    cargo = tomllib.loads(CARGO_TOML.read_text(encoding="utf-8"))
    cargo_version = cargo.get("workspace", {}).get("package", {}).get("version")
    if cargo_version != version:
        errors.append(f"Cargo.toml workspace.package.version={cargo_version!r}, expected {version!r}")

    lock = tomllib.loads(CARGO_LOCK.read_text(encoding="utf-8"))
    for package in lock.get("package", []):
        name = package.get("name", "")
        if name == "inputkey" or name.startswith("inputkey-"):
            if package.get("version") != version:
                errors.append(
                    f"Cargo.lock {name}={package.get('version')!r}, expected {version!r}"
                )

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    for key in ("version", "version_name"):
        if manifest.get(key) != version:
            errors.append(f"Chrome manifest {key}={manifest.get(key)!r}, expected {version!r}")

    for path, project in ((FCITX, "inputkey-fcitx5"), (IBUS, "inputkey-ibus")):
        text = path.read_text(encoding="utf-8")
        match = re.search(
            rf"project\({re.escape(project)} VERSION (\d+\.\d+\.\d+)",
            text,
        )
        actual = match.group(1) if match else None
        if actual != version:
            errors.append(f"{path.relative_to(ROOT)} version={actual!r}, expected {version!r}")

    plist = plistlib.loads(PLIST.read_bytes())
    for key in ("CFBundleShortVersionString", "CFBundleVersion"):
        if plist.get(key) != version:
            errors.append(f"macOS {key}={plist.get(key)!r}, expected {version!r}")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Keep VERSION as InputKey's release-version SSOT.")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="write VERSION into platform metadata")
    mode.add_argument("--check", action="store_true", help="verify platform metadata (default)")
    args = parser.parse_args()

    try:
        version = canonical_version()
        if args.write:
            write(version)
        errors = collect(version)
    except (OSError, ValueError, RuntimeError, tomllib.TOMLDecodeError, plistlib.InvalidFileException) as exc:
        print(f"Version SSOT error: {exc}")
        return 1

    if errors:
        print("Version SSOT mismatch:")
        for error in errors:
            print(f"- {error}")
        print("Run: python Bootstrap/Build/sync-version.py --write")
        return 1

    print(f"Version SSOT passed: VERSION={version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
