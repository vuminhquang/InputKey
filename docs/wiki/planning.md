# InputKey active plan

Checkpoint: 8

## Current objective

The authorized InputKey v5.2.16 Shift+Space behavioral maintenance release is COMPLETE at the product source, cross-platform GitHub Release, and previously installed Windows portable targets. The historical v5.2.15 release evidence is retained in [the release archive](archive/2026-10-20-v5.2.15-release-evidence.md). A GitHub Release does not implicitly update either browser store.

## Verified release evidence (2026-10-26)

- Behavior: Shift+Space uses normal SpaceBoundary when `displayed == raw`; otherwise it preserves RawBoundary (raw physical keystrokes, without adding Space). No modifier timing or fresh-Shift logic; root/language composition contracts remain intact.
- Exact source: annotated tag `v5.2.16`, peeled commit `b054407d02fb9b89c24125113a32e9cbcb75340e`. All platform version metadata synchronized to `5.2.16`.
- Local gates: full Rust workspace tests, Clippy all targets with denied warnings, formatting, architecture, version/handoff, Chrome UI 19/19, Windows Hook/TSF/startup, Linux adapters, C ABI optimized build and smoke succeeded. Windows and Chromium release packaging succeeded.
- [Main CI run 40289988287](https://github.com/vuminhquang/InputKey/actions/runs/40289988287): Linux/Web, Windows and macOS passed.
- [Tag CI run 40290503321](https://github.com/vuminhquang/InputKey/actions/runs/40290503321): Linux/Web, Windows, macOS and GitHub Release all passed.
- [GitHub Release v5.2.16](https://github.com/vuminhquang/InputKey/releases/tag/v5.2.16) is publicly published (neither draft nor prerelease), with four ZIP packages verified against release asset metadata:
  - Chromium: `InputKey-Chromium-5.2.16.zip`, 534112 bytes, SHA-256 `2f5a2807ca3f204469205e4f5aa67cf7e8b2135b0e22d6cc97ca9e2ff2fb5dd2`.
  - Linux: `InputKey-Linux-x86_64-5.2.16.zip`, 682086 bytes, SHA-256 `1bc97b04369619220a07516c0f73dce6f9ad66ee031cb8afbbed4fa0cf8bff46`.
  - macOS: `InputKey-macOS-5.2.16.zip`, 593342 bytes, SHA-256 `3592f101f7a78f43f0e1d4f03e8d6d9e1e839b6672d0c3a40fe339c005872895`.
  - Windows: `InputKey-Windows-5.2.16.zip`, 2637507 bytes, SHA-256 `e4f4cc7200458b45e4e2f9afd4217b88f4e8c1d98fcc415de0737a06af1236a6`.
- Windows portable deployments on the previously approved physical targets were verified separately (installed package files matching local ZIP 12/12, running target processes, version and TSF binding). Account-specific locations, device status and private recovery paths are retained only in the owner-side deployment receipt.

## Next safe action

No further GitHub release action for v5.2.16. Chrome Web Store and Edge Add-ons submissions are separate operations, not proven by GitHub publication or a locally unpacked extension. Real typing after prolonged lock/unlock remains a distinct interactive acceptance from packaging or TSF registration.
