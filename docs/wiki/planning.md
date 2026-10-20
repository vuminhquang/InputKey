# InputKey active plan

Checkpoint: 6

## Current objective

Release the verified Vietnamese ordered-Telex and phonotactic fix as InputKey v5.2.15 from this **canonical product Git**, then deploy the exact official Windows release ZIP to two owner-authorized targets: the owner's two explicitly identified existing portable installations (DESKTOP and VNN). The owner explicitly authorized both release and deployment on 2026-10-20. Neither the private owner Git nor any reconstructed source mirror participates in product authoring or public transport.

## Intended behavior and scope

- `barn`/`banr` -> `bản`, `dudowcj` -> `được`, `quees` -> `quế`; complete ordered Telex candidates pass Vietnamese orthographic/phonotactic constraints before an English spelling match can override them. The invalid speculative `quêu` is rejected so `queue` stays raw, including without an English dictionary. Physical base-character order and explicit RawBoundary/cancel remain authoritative.
- A complete valid Vietnamese reading may now win without an explicit tone key, e.g. `dud` -> `đu`; users can explicitly request literal raw text. This behavior is intentional and must be explained in release notes.
- One product source, no extra vocabulary dataset, no application startup/timeout workaround.

## Verified baseline / planned acceptance

- The former local source candidate is `4115b7bcfb09643ed7d331075357b663e6e96e04`, parent public main `5bbf9cb04ac13a7e66399ff3df232de325bbdf82`; the feature's 27/27 Vietnamese regressions, Rust workspace, clippy, fmt and source contract checks were green with pre-release metadata.
- `VERSION=5.2.15` is a **release candidate** until the final source passes tests and a GitHub version-tag CI run creates and verifies the actual four ZIP assets. Product release, Chrome/Edge store submission and device deployment are separate outcomes.

## Next safe action

Run final local source gates on the versioned product tree, fast-forward canonical public main after clean diff review, and verify remote main; only then push a version tag and inspect the tag-triggered Linux/Web, Windows, macOS and GitHub Release jobs. Fetch the official Windows ZIP, validate package VERSION and SHA256 on both devices, then run the checked-in portable deploy script against each exact pre-verified destination with versioned runtime preservation and independent installed-process/TSF verification. Never claim store submission or physical post-lock/unlock typing without direct evidence.
