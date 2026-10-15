# InputKey release acceptance

This file owns terminal acceptance for a named InputKey release. The active plan tracks progress but cannot substitute for these gates. Optional/future features are not release blockers unless expressly included in an approved scope.

A source/packaging release is complete only if:
1. Scope and VERSION are explicit, behavior/contracts for all changed languages and hosts are specified, and no unresolved release-blocking decision is silently assumed.
2. Affected behavior tests pass against the actual final source state; the Rust workspace full regression, formatting, clippy, architecture/version checks and relevant adapter contracts are green.
3. Linux/Web, Windows and macOS package/build checks succeed for that revision (locally or in authoritative CI); generated artifacts correspond to the intended VERSION, with third-party/privacy notices and a reviewed secret boundary.
4. Git commit and intended remote revision/tag are verified separately from the build results. A tag-triggered GitHub Release is complete only after its workflow and published artifact identity are confirmed.

A STORE release additionally requires its own submitted package identity, listing/privacy compliance and authoritative store review/publish status. A DEPLOYMENT additionally requires documented target-specific preconditions, exact artifact, safe recovery/rollback, and live functional checks on the installed target. Neither store approval nor deployment follows automatically from a source release. If external status cannot be checked, it remains UNKNOWN.

A maintenance/documentation migration is complete when project-native entrypoint, canonical owners, one active plan, recoverable evidence, working test/playbook instructions and a clean document-navigation check are verified; it does not certify a product release or change installed runtime behavior.
