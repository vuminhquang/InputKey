# InputKey active plan

Checkpoint: 2

## Current objective

Maintain this repository's standalone public engineering handoff and preserve InputKey v5.2.12 behavior. This file is the only active execution-state owner. Long-lived facts live in their linked semantic owner documents.

## Observed baseline

- Base Git revision at migration start: `72fa4fe483efbfe883369e6fc97799cae3ae9585`; the worktree was clean at inspection.
- Build and release definitions are in `.github/workflows/build.yml`. No new deployment or store publication is authorized by this documentation change.
- Earlier verification applies only to the revision and environment where it was observed.

## Work and verification

- [x] Public wiki navigation and cold-start discovery: 15 Markdown files, verified by `python Tests/HandoffTests/check_handoff.py`.
- [x] Architecture and version checks: `check-architecture.py`, `sync-version.py --check` green on the documentation snapshot.
- [ ] Review exact staged diff and private-data exposure before an authorized commit/push. Remote publication is not implied.

## Next safe action

Inspect and stage only the public handoff/CI changes; verify the public repository revision. Keep source-release, store and installed runtime outcomes separate.
