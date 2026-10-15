# InputKey active plan

Checkpoint: 3

## Current objective

Maintain this repository's standalone public engineering handoff and preserve InputKey v5.2.12 behavior. This file is the only active execution-state owner. Long-lived facts live in their linked semantic owner documents.

## Observed baseline

- Base Git revision at migration start: `72fa4fe483efbfe883369e6fc97799cae3ae9585`; the worktree was clean at inspection.
- Build and release definitions are in `.github/workflows/build.yml`. No new deployment or store publication is authorized by this documentation change.
- Earlier verification applies only to the revision and environment where it was observed.

## Work and verification

- [x] Public wiki navigation and cold-start discovery: 15 Markdown files, verified by `python Tests/HandoffTests/check_handoff.py`.
- [x] Architecture and version checks: `check-architecture.py`, `sync-version.py --check` green on the documentation snapshot.
- [x] Reviewed only the handoff/CI staged diff and checked for private-data exposure. Public-safe documentation committed locally as `cbf0a9e3a108b95d66d4e85881ac1c6855ac51f7`.
- [ ] Public remote is not yet verified as updated to this migration revision; a local commit is not remote publication.

## Next safe action

Keep this public repository independently navigable. Verify the exact public remote and approval before any remote publication; do not infer source-release, store or installed runtime outcomes from documentation work.
