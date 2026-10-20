# InputKey active plan

Checkpoint: 5

## Current objective

Preserve and review the Vietnamese phonotactic composition improvement in this local product-source branch. This is an unreleased engineering change, not a new VERSION, publication, store submission or device deployment. Current public release remains v5.2.14.

## Observed source and verification

- Local branch: `fix/vietnamese-phonotactics-unreleased`, based on public `5bbf9cb04ac13a7e66399ff3df232de325bbdf82` with `VERSION=5.2.14` unchanged.
- Scope: Vietnamese language FSM, runtime regression tests, and canonical composition contract. Telex complete ordered parsing is evaluated by orthographic/phonotactic constraints instead of a mandatory tone-key heuristic. Explicit literal control retains its precedence; `queue` stays literal without an English dictionary.
- Local Windows source evidence: version check, architecture check, handoff check, fmt, full Rust workspace tests, Clippy all targets with warnings denied and diff check returned exit 0 (`L2_LOCAL_NO_VERSION_BUMP_GATES_GREEN`); 27/27 Vietnamese integration regressions passed.
- This local source result does not prove Linux/macOS CI, published package, store approval or live typing behavior.

## Next safe action

Keep the change on the local product development branch. Do not change VERSION, push public main, tag, publish a release, upload stores or deploy until the owner explicitly authorizes the intended release scope. Product code and wiki changes are authored and reviewed in this repository only; source copies elsewhere are not product authorities.
