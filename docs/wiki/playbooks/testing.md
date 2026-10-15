# InputKey testing

Run tests from repository root. Focused edits start with their smallest relevant executable check; shared semantic contract changes additionally run affected language, runtime and adapter contracts. Do not use an unrelated full-suite pass in place of focused failure diagnosis.

Examples of existing focused checks:
- Engine: `cargo test -p inputkey-runtime`; individual tests may use the Rust test-name filter.
- Architecture: `python Bootstrap/Build/check-architecture.py`.
- Chromium UI: `python Bootstrap/Build/check-chrome-ui.py`; JS extension tests: `node Implementations/Chrome/extension/content_test.cjs` (requires built WASM where applicable).
- Windows contracts: `python Bootstrap/Build/check-windows-hook.py`, `python Bootstrap/Build/check-windows-tsf.py`, `python Bootstrap/Build/check-windows-startup.py`.
- Linux adapters: `python Bootstrap/Build/check-linux-adapters.py`.

Final source-release gate after targeted checks: `python Bootstrap/Build/sync-version.py --check`, `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, architecture/platform checks, `git diff --check` and a secret/staged-diff review. Run platform-specific builds on the proper host/CI; see `.github/workflows/build.yml`.

Existing Rust integration test and Chromium harness locations are recorded in [repo-layout](../architecture/repo-layout.md). New tests should consume public semantic contracts externally and avoid test-only dependencies in core production logic; movement of legacy tests is an independent refactor requiring green CI. A local check does not establish cross-platform CI or a deployed result.
