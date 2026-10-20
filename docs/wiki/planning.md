# InputKey active plan

Checkpoint: 7

## Current objective

The approved InputKey v5.2.15 source and packaging release is COMPLETE. Continue ordinary product maintenance from this independent canonical product Git. A GitHub Release does not implicitly submit a Chrome/Edge store update or deploy any user machine; those are separate authorization/verification scopes.

## Verified release evidence (2026-10-20)

- Exact tagged source: `v5.2.15` (annotated, peeled commit `5a730270e5f2a5f36ab803da9870ec0d88cc9627`). The release incorporates previously verified Vietnamese phonotactic/ordered-Telex changes from `4115b7bcfb09643ed7d331075357b663e6e96e04` with synchronized VERSION 5.2.15, without a second product-source mirror.
- Behavior contract: completed ordered valid Vietnamese interpretations override a coincident English raw word, e.g. `barn`/`banr` -> `bản`, `dudowcj` -> `được`, `quees` -> `quế`. Incompatible speculative `quêu` is rejected so `queue` remains raw even without an English lexicon; base-letter order, explicit raw/cancel and equal-ranked ambiguity protections stay intact. Valid completed Vietnamese may also win without a tone (e.g. `dud` -> `đu`), with explicit literal control available.
- Local source acceptance: 27/27 Vietnamese integration regressions, full Rust workspace, fmt, Clippy all targets with warnings denied, handoff/architecture/version, Chrome UI, Windows hook/TSF/startup, C ABI optimized build and smoke succeeded, marker `BARN_5_2_15_L2_GATES_GREEN`. Local Windows packaging script also completed successfully.
- GitHub main pipeline [39401412702](https://github.com/vuminhquang/InputKey/actions/runs/39401412702): Linux/Web, Windows and macOS successful. Version-tag pipeline [39401795775](https://github.com/vuminhquang/InputKey/actions/runs/39401795775): Linux/Web, Windows, macOS and GitHub Release all successful. [v5.2.15 release](https://github.com/vuminhquang/InputKey/releases/tag/v5.2.15) published (not draft or prerelease) with exactly four ZIPs:
  - Chromium: `InputKey-Chromium-5.2.15.zip`, 533941 bytes, SHA-256 `9001a35df2ea16754678db21b48c304ed967c98ce7c415d89b40078caf02ee5b`.
  - Linux: `InputKey-Linux-x86_64-5.2.15.zip`, 681750 bytes, SHA-256 `41d479c6c56507bde5ed8608c08ac0f1065033daac1e328c063b3d823505eacb`.
  - macOS: `InputKey-macOS-5.2.15.zip`, 592794 bytes, SHA-256 `136f01f3299389c503d778512249c8f6d054aa7dcb2f490314a7bba652d287b9`.
  - Windows: `InputKey-Windows-5.2.15.zip`, 2637825 bytes, SHA-256 `0dad2824e1e3d6f4c62625778f4cd078affbff3ed4df7cba2c872c6818735fc4`.
- Physical installed-system evidence, individual machine paths and account-specific status are not part of this public product planning document; release packaging and deployment have independent acceptance scopes.

## Next safe action

No further publication action for this completed GitHub release. Store updates, physical interactive typing after prolonged lock/unlock and future product changes each require their own scope and evidence. Do not mistake a deployed binary, GitHub tag or CI result for an unobserved store approval or live keyboard acceptance.
