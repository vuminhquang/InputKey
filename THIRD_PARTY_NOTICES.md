# Third-party notices

InputKey's Rust transformation engine and platform adapters are project code.

## SCOWL / Hunspell English word list

The optional English collision resolver uses a compact on-device Bloom filter generated from lowercase entries in the SCOWL-derived \`en_US\` Hunspell dictionary. It contains no user data and is used only to choose between ambiguous literal/cancel candidates such as \`password\` / \`pasword\` and \`tesst\` / \`test\`.

SCOWL collective work Copyright 2000-2011 Kevin Atkinson. Permission to use, copy, modify, distribute and sell these word lists, associated scripts, output created from the scripts, and documentation for any purpose is granted without fee, provided the copyright notice and permission notice appear in copies and supporting documentation. The material is provided "as is" without warranty. SCOWL also incorporates sources with their own permissive/public-domain notices; see the SCOWL distribution for full provenance.

## Rust dependencies

InputKey uses Rust crates recorded in \`Cargo.lock\`, including \`windows-sys\` for the Windows implementation. Their upstream license metadata and notices remain authoritative for those dependencies.

## Operating-system frameworks

The native adapters link against operating-system or distribution-provided frameworks such as Win32, InputMethodKit/AppKit, Fcitx5, and IBus. These frameworks are not vendored into this repository.
