# State machine contract

The language-independent root composition machine lives in `Operators` and is compiled to WASM for Chrome together with the bundled language children. This file documents behavior; it does not define rules.

Chrome/Edge uses the same composition-boundary semantics as the native adapters. Shift+Space is a fixed one-shot raw boundary; it is not a persistent mode or a configurable shortcut.

## Vietnamese child modes

| Mode | Meaning |
| --- | --- |
| `START` | no active token |
| `VI_CANDIDATE` | token is still a viable Vietnamese candidate |
| `RAW_LOCKED` | token has deterministically fallen back to literal/raw typing, including a committed repeat-cancel checkpoint |

These modes belong to the Vietnamese child, not to the root machine. The Vietnamese candidate also tracks onset/nucleus/coda/pending-shape/pending-validation/complete/dead phases. Each logical input event enters one FSM transition. Backspace restores engine state rather than reparsing visible DOM text.

## Repeat-cancel contract

Repeat-to-cancel is a built-in feature, not an option. Older stored
`doubleCancel: false` values cannot disable it; `docss` produces `docs`.

Some physical sequences cannot reveal intent by themselves. `P A S S`, for example, can mean English `PASS` or Telex `PAS` where the second `S` explicitly cancels the acute modifier.

The current engine uses a deterministic, expressible contract:

- the second modifier commits immediately to a collapsed `RAW_LOCKED` checkpoint; explicit cancel therefore wins at token end (`pass` keystrokes → `pas`) and later keys continue from that checkpoint (`u-r-r-l → url`)
- a third repeated modifier appends one literal modifier after that checkpoint (`passs` keystrokes → `pass`)
- the physical keystroke stream is retained separately for Auto Restore; only later complete English lexical evidence may recover it (`password → password`), while the cancelled modifier itself is no longer pending
- Telex `s/f/r/x/j` and VNI tone repeats all use the same committed-cancel transition

Legacy `CancelPreference` values cannot override an immediate repeat-to-cancel gesture.

## Ordering-independent shapes

The FSM supports context-aware late transitions such as:

- `nuawx → nữa`
- `ddaya → đây`
- `duowjcd → được`
- `nhieue → nhiêu`
- `chueyern → chuyển`
- `dduocwj → được`

High-confidence correction only rearranges or applies intents that were actually typed; it does not invent a missing tone key. The resolver runs once from the root-owned Space boundary, not during live key transitions. Live Telex remains sequential, while explicit repeat-cancel remains an immediate character-level transition.

Repeating the immediately preceding shape/stroke operation cancels it even when
its target precedes the coda: `dataa → data`, `dayaa → daya`, `banww → banw`.
Backspace restores the state before the cancel. A further repeated key is literal. The `[` → `ư` and `]` → `ơ` shortcuts follow the same rule: `[[` → `[` and `]]` → `]`.

## Ending a composition

The root owns explicit lifecycle phases: `Idle`, `Composing`, `CorrectionBoundary`, `RawBoundary`, `NaturalBoundary`, `MouseBoundary`, and `FinalizeBoundary`. A boundary event first moves the root into its boundary state; that state then performs its commit policy and returns the root to `Idle`.

Space enters `CorrectionBoundary` once, lets the active language perform its optional correction, then finalizes and inserts a normal space. Punctuation uses `FinalizeBoundary` without correction. Enter, Tab, navigation keys, Delete, and Insert enter `NaturalBoundary`, commit the currently displayed text, return to `Idle`, and let the same physical key continue to the application.

A pointer/caret relocation enters the separate `MouseBoundary` state before the default pointer action changes the caret or selection. `MouseBoundary` and `NaturalBoundary` are different semantic states, but both call the root's private displayed-text commit-and-reset primitive. For example, if `dd` displays `đ`, clicking elsewhere first leaves `đ` committed at the old caret and only then relocates the caret.

Shift+Space enters `RawBoundary`, commits only the token's physical keystream, returns to `Idle`, and consumes the Space keystroke. For example, `refer` may display `rể`, while Shift+Space commits `refer`. Escape restores the literal token in-place and continued typing stays literal until the next boundary.

Only one input method should own a field at a time. When the browser extension owns composition in a remote or WSLg browser, disable any second IME for that same field to avoid double conversion.
