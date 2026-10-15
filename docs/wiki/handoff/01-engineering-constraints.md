# InputKey engineering constraints

- One language-neutral root composition FSM owns semantic input routing and lifecycle. Language packs interpret root transitions through the stable semantic contract; host adapters must not implement duplicate language decisions.
- Boundary owns external input/output shapes; CoreAbstractions owns semantic state and ports; Operators owns root orchestration; concrete language and platform mechanisms live in Implementations; Bootstrap wires executable entrypoints.
- Dependencies must point toward stable contracts. The core must not depend on OS/browser mechanisms or vendor libraries. Cross-module meaning belongs to its actual semantic owner, not an ad hoc catch-all module.
- Treat architectural roles as responsibilities, not required empty directories. Retain the currently buildable top-level Cargo layout until any relocation is scoped and verified separately.
- Platform-specific transports must preserve the root-to-language contract. Windows TSF is primary; compatibility is a separately bounded fallback.
- Keep user typing local and honor the existing extension privacy contract.
- Review edits using [the architecture map](../architecture/README.md), [feature contracts](../features/README.md), and [testing playbook](../playbooks/testing.md); never use a green test as evidence of approval for an unrelated design choice.
