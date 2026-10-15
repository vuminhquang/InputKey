# Composition contract

The root accepts semantic input events: Character, SpaceBoundary, PunctuationBoundary, CaretMoveBoundary, ShortcutBoundary, CompositionControl, RawBoundary and Lifecycle. It changes root state before synchronously emitting RootTransition to the selected language FSM.

Language-specific FSMs own text interpretation. Space and punctuation are distinct root states and invoke the language's boundary policy before the original delimiter is emitted. Navigation/mouse and shortcuts commit the displayed composition before the original host input proceeds. AltGraph remains character input. Backspace/Escape control composition; Shift+Space commits raw physical input once and consumes Space. Lifecycle events terminate/reset the active context.

Vietnamese supports Telex, Simple Telex and VNI, plus modifier reordering correction at word boundaries without inventing tone input. Live typing remains sequential and explicit repeat-cancel remains immediate. Tests and the current behavioral overview are in [README](../../../README.md), `Bootstrap/Runtime/tests/vietnamese_regression.rs`, and the language FSM implementation. Any changed semantic event requires core, host adapter and language-pack compatibility verification together.
