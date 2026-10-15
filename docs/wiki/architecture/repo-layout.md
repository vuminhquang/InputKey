# InputKey source and knowledge ownership

The current buildable Cargo workspace is rooted at `Cargo.toml`. Its semantic roles are currently physically represented at root level; changing those paths is a separate verified source migration, not an incidental documentation change.

| Ownership | Canonical source |
| --- | --- |
| Engine input/output contract | `Boundary/` and its README |
| Semantic types, events and ports | `CoreAbstractions/` and its README |
| Composition, session and replay orchestration | `Operators/` and its README |
| Language-specific and host mechanisms | `Implementations/` and module-local documentation |
| Runtime/ABI composition and build utilities | `Bootstrap/` and `Bootstrap/Build/` |
| Product overview | `README.md` |
| Source/platform architecture | `ARCHITECTURE.md` |
| Feature semantics | `docs/wiki/features/` |
| Repeatable engineering procedures | `docs/wiki/playbooks/` |
| Current objective, blockers, evidence and safe next action | `docs/wiki/planning.md` only |
| Completion criteria for a stated scope | `docs/wiki/project-completion.md` only |
| Agent entrypoint | `AGENTS.md` → `docs/wiki/handoff/README.md` |

When asked to record an approved fact: locate its current semantic owner, edit that document, and change an index only if navigation changed. Do not use planning or a README index as a universal memory dump. Unapproved proposals remain identified as proposals; missing permission remains UNKNOWN. Do not store credentials in Git.

The architecture, features and playbooks indexes are Markdown-canonical because their current consumers navigate links. Only adopt a typed JSONL master if a real checked-in consumer needs typed record queries and a tested generator, validator and schema exist; never maintain two independent masters.

Test code currently includes Rust integration tests in `Bootstrap/Runtime/tests` and browser tests in `Implementations/Chrome/extension/content_test.cjs`. These are observed legacy locations, not permission to put new test-only behavior inside production modules. Separate test relocation requires its own build/CI validation. Build outputs under `target/`, `dist/` are ignored.

This project repository is public. All tracked documents and commit messages must be suitable for its public audience; do not record private workspace paths, secret values, owner-only operational metadata or unaudited external account details here.
