"""Check that the InputKey public handoff is self-contained and navigable."""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs" / "wiki"
REQUIRED = [
    ROOT / "AGENTS.md", DOCS / "handoff" / "README.md",
    DOCS / "handoff" / "01-engineering-constraints.md",
    DOCS / "handoff" / "02-execution-protocol.md",
    DOCS / "planning.md", DOCS / "project-completion.md",
    DOCS / "architecture" / "README.md", DOCS / "architecture" / "repo-layout.md",
    DOCS / "features" / "README.md", DOCS / "playbooks" / "README.md",
]
errors = []
for p in REQUIRED:
    if not p.is_file():
        errors.append("missing: " + str(p.relative_to(ROOT)))
if len(list(DOCS.glob("planning.md"))) != 1:
    errors.append("exactly one active plan must exist")
markdown = [ROOT / "AGENTS.md", *DOCS.rglob("*.md")]
for p in markdown:
    contents = p.read_text(encoding="utf-8")
    # README navigation is part of this repository's interface.
    for raw in re.findall(r"(?<!!)\[[^\]]+\]\(([^)]+)\)", contents):
        link = raw.split("#", 1)[0].strip()
        if not link or re.match(r"^[a-z]+://", link, re.I) or link.startswith("mailto:"):
            continue
        target = (p.parent / link).resolve()
        if not target.is_file():
            errors.append(f"broken link: {p.relative_to(ROOT)} -> {raw}")
    if re.search(r"(?i)[A-Z]:\\(?:repos|users)\\", contents) or ".skills\\" in contents:
        errors.append(f"local-only path in shared document: {p.relative_to(ROOT)}")
if errors:
    print("\n".join(errors))
    sys.exit(1)
print(f"Public handoff passed: {len(markdown)} Markdown files checked.")
