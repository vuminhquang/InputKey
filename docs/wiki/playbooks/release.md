# Version, packages and publishing

`VERSION` and root `Cargo.toml` are version sources validated with `python Bootstrap/Build/sync-version.py --check`. Read the checked-in `.github/workflows/build.yml` for the exact release pipeline; only version tags (`v*`) trigger the GitHub Release job, after Linux/Web, Windows and macOS jobs. CI completion and published release require authoritative verification, not assumption.

Existing package scripts: `Bootstrap/Build/build-chrome.ps1` / `.sh`, `build-windows.ps1`, `build-linux.sh`, `build-macos.sh`. Build outputs are under ignored `dist/`. Building does not install, publish a store entry or deploy a user machine.

For a Windows portable installation, the existing `Bootstrap/Build/deploy-windows-portable.ps1` requires explicit Package and Destination arguments, stages a versioned runtime and verifies TSF binding; it mutates processes/registration and therefore requires a separately authorized exact target and pre-deployment recovery/verification plan. Do not run it for documentation migration or as a generic smoke check. Other target installs/store submissions similarly require their own authorized procedure and final live verification.

Before claiming any release complete, use [project-completion](../project-completion.md). Record actual artifact revisions and independent acceptance evidence; leave unavailable store, CI or machine state UNKNOWN.
