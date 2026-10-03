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

High-confidence correction only rearranges or applies intents that were actually typed; it does not invent a missing tone key. At Space/Punctuation boundaries it also preserves the physical initial prefix: a typed consonant onset cannot be consumed as a modifier, and a vowel-initial token can only remain in the same initial vowel family (`a/ă/â`, `e/ê`, `o/ô/ơ`, `u/ư`, `i`, `y`). The resolver runs once from the root-owned boundary, not during live key transitions. Live Telex remains sequential, while explicit repeat-cancel remains an immediate character-level transition.

Repeating the immediately preceding shape/stroke operation cancels it even when
its target precedes the coda: `dataa → data`, `dayaa → daya`, `banww → banw`.
Backspace restores the state before the cancel. A further repeated key is literal. Bracket keys are ordinary punctuation and are not Vietnamese shape shortcuts.

## Ending a composition

The root classifies input into semantic events and enters the corresponding state before emitting a synchronous `RootTransition` to the active language FSM.

`SpaceBoundary` and `PunctuationBoundary` are distinct states because their causes and delimiters are different. They intentionally share the same language boundary policy: Vietnamese may perform its correction pass for either event, then the root appends the original delimiter. Thus `dduwocj ` and `dduwocj.` both resolve to `được` before their delimiter is emitted.

Mouse clicks and Enter/Tab/navigation/Delete/Insert all enter `CaretMoveBoundary`; the `cause` distinguishes Mouse, Enter, Left, and so on. The language commits the displayed text and ends composition before the original caret-moving event continues.

Ctrl/Alt/Meta command chords enter `ShortcutBoundary`, commit displayed composition, and continue to the application unchanged. AltGraph is excluded from shortcut classification. Backspace/Escape are `CompositionControl` events. Shift+Space enters `RawBoundary`, commits the physical keystream, and consumes the Space key. Focus/context/enable/language changes are `Lifecycle` events.

Language packs do not expose separate key/backspace/finalize/correction operations to the root. They keep their own language FSM internally and consume the single RootTransition stream.


Only one input method should own a field at a time. When the browser extension owns composition in a remote or WSLg browser, disable any second IME for that same field to avoid double conversion.
