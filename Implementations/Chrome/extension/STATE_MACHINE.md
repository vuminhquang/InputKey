# State machine contract

The FSM lives in the repository `Operators` layer and is compiled to WASM for Chrome. This file documents behavior; it does not define rules.

The Chrome/Edge extension lets users configure the “Hoàn tác dấu của từ” shortcut in its popup. It defaults to Ctrl+Space on Windows and in Chrome/Edge; the extension matches the recorded physical key code and exact modifiers before issuing `LiteralizeToken`.

## Main modes

| Mode | Meaning |
| --- | --- |
| `START` | no active token |
| `VI_CANDIDATE` | token is still a viable Vietnamese candidate |
| `RAW_LOCKED` | token has deterministically fallen back to literal/raw typing, including a committed repeat-cancel checkpoint |

The Vietnamese candidate also tracks onset/nucleus/coda/pending-shape/complete/dead phases. Backspace restores snapshots rather than reparsing visible DOM text.

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

These are grammar-validated transitions, not word-specific exceptions.

Repeating the immediately preceding shape/stroke operation cancels it even when
its target precedes the coda: `dataa → data`, `dayaa → daya`, `banww → banw`.
Backspace restores the state before the cancel. A further repeated key is literal. The `[` → `ư` and `]` → `ơ` shortcuts follow the same rule: `[[` → `[` and `]]` → `]`.

## Ending a composition

Space/punctuation finalize with Auto Restore. Cursor movement instead keeps the
displayed text and resets history: `data`, Right, Space leaves `dât`. Navigation
is never prevented by this operation. Escape restores the literal token and
continued typing stays literal until the next boundary.

For Chrome in WSLg/RDP, the Windows adapter passes physical keys through and this
extension owns composition. Do not enable a second Linux IME in the same field.
