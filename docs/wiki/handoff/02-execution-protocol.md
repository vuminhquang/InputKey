# InputKey execution and recovery

The single active execution file is [planning.md](../planning.md). [project-completion.md](../project-completion.md) independently owns the acceptance boundary. Archives are historical evidence, never a second active plan.

For each approved, coherent slice: **recover → reconcile → execute → verify → persist → inspect/stage only owned files → commit/push only to an authorized target → verify actual destination → continue the next safe in-scope slice**. A successful slice or a status message is not automatic project completion. Full regression is a distinct final gate, not the default inner loop.

On recovery inspect actual Git revision and dirty changes, active plan, relevant owners, earlier verification's exact source state, inherited unresolved tasks and any interrupted external operation. Responsibility for an unresolved project task survives a session/agent switch. Attribute historical actions only when supported by evidence; investigate accessible Git/checkpoints rather than disclaiming earlier work.

Distinguish action outcome from permission: a test, commit, push or deployment never retrospectively proves a user approved the technical or scope decision. Record a missing/conflicted authorization as UNKNOWN until resolved, without silently endorsing or reverting existing work.

Persist approved consequential decisions to the correct canonical semantic owner *before relying on them*. Planning holds a compact checkpoint: increasing sequence, objective, canonical pointers, observed base revision, dirty-state description, last verified claim/evidence and the safe next action. One orchestrator writes it; a sequence number alone is not a file lock. If interrupted during archival, restore exactly one active plan from the preserved archive and prepared successor before proceeding.

For non-idempotent external effects (deploy, migration, publication, externally sent messages), first persist operation ID, exact target, preconditions, read-only reconciliation method and pending outcome, without secrets. An interrupted or inconclusive outcome is UNKNOWN: inspect authoritative state before considering retry; never retry blindly or claim success from command exit alone. Do not perform deployment without the project-specific playbook and authorization.

Classify progress precisely: Working, Committed, locally packaged, remotely published and Deployed are separate evidence-backed states. Stop only on verified assigned completion, owner stop, missing permission, genuine blocker or unsafe/unknown external effect. Preserve unrelated worktree changes and never bulk-stage them.
