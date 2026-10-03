use std::sync::{Arc, Mutex};

use inputkey_boundary::{InputEvent, InputType};
use inputkey_core_abstractions::{
    CaretMoveCause, CompositionControl, CoreEvent, CoreEventType, EventPublisher,
    LanguageMachinePort, LexiconPort, LifecycleEvent, RootInput, RootPhase, RootTransition,
};
use inputkey_language_vietnamese::{Machine, Mode, Options, Phase};
use inputkey_operators::{replay, Machine as RootMachine, Session};

fn phase_for(input: RootInput) -> RootPhase {
    match input {
        RootInput::Character(_) => RootPhase::Composing,
        RootInput::SpaceBoundary => RootPhase::SpaceBoundary,
        RootInput::PunctuationBoundary(_) => RootPhase::PunctuationBoundary,
        RootInput::CaretMoveBoundary(_) => RootPhase::CaretMoveBoundary,
        RootInput::ShortcutBoundary => RootPhase::ShortcutBoundary,
        RootInput::CompositionControl(_) => RootPhase::CompositionControl,
        RootInput::RawBoundary => RootPhase::RawBoundary,
        RootInput::Lifecycle(_) => RootPhase::Lifecycle,
    }
}

trait SemanticDriver {
    fn semantic(&mut self, input: RootInput) -> String;
    fn displayed(&self) -> String;
    fn physical(&self) -> String;
    fn active(&self) -> bool;

    fn character(&mut self, character: char) -> String {
        self.semantic(RootInput::Character(character))
    }

    fn space_boundary(&mut self) -> String {
        self.semantic(RootInput::SpaceBoundary)
    }

    fn punctuation_boundary(&mut self, delimiter: char) -> String {
        self.semantic(RootInput::PunctuationBoundary(delimiter))
    }

    fn caret_move(&mut self, cause: CaretMoveCause) -> String {
        self.semantic(RootInput::CaretMoveBoundary(cause))
    }

    fn shortcut_boundary(&mut self) -> String {
        self.semantic(RootInput::ShortcutBoundary)
    }

    fn control(&mut self, control: CompositionControl) -> String {
        self.semantic(RootInput::CompositionControl(control))
    }

    fn raw_boundary(&mut self) -> String {
        self.semantic(RootInput::RawBoundary)
    }

    fn lifecycle(&mut self, event: LifecycleEvent) -> String {
        self.semantic(RootInput::Lifecycle(event))
    }
}

impl SemanticDriver for RootMachine {
    fn semantic(&mut self, input: RootInput) -> String {
        self.dispatch(input)
    }

    fn displayed(&self) -> String {
        self.state().rendered
    }

    fn physical(&self) -> String {
        self.state().raw
    }

    fn active(&self) -> bool {
        !self.state().raw.is_empty()
    }
}

impl SemanticDriver for Machine {
    fn semantic(&mut self, input: RootInput) -> String {
        let from = if self.state().raw.is_empty() {
            RootPhase::Idle
        } else {
            RootPhase::Composing
        };
        self.on_transition(RootTransition {
            from,
            to: phase_for(input),
            input,
        })
        .text
    }

    fn displayed(&self) -> String {
        self.state().rendered
    }

    fn physical(&self) -> String {
        self.state().raw
    }

    fn active(&self) -> bool {
        !self.state().raw.is_empty()
    }
}

struct Words;

impl LexiconPort for Words {
    fn has_word(&self, word: &str) -> bool {
        [
            "software",
            "network",
            "deterministic",
            "password",
            "coffee",
            "test",
            "ask",
            "after",
            "url",
            "exe",
            "ajax",
        ]
        .contains(&word)
    }
}

#[derive(Clone)]
struct EventCollector {
    events: Arc<Mutex<Vec<CoreEvent>>>,
}

impl EventPublisher for EventCollector {
    fn publish(&self, event: CoreEvent) {
        self.events.lock().expect("event lock").push(event);
    }
}

fn options(method: &str, simple_telex: bool, auto_restore: bool) -> Options {
    Options {
        method: method.to_owned(),
        simple_telex,
        auto_restore,
        smart_correction: true,
        double_cancel: true,
        cancel_preference: String::new(),
    }
}

fn run(raw: &str, options: Options) -> String {
    let mut machine = root(options);
    for key in raw.chars() {
        machine.character(key);
    }
    machine
        .space_boundary()
        .strip_suffix(' ')
        .unwrap_or_default()
        .to_owned()
}

fn root(options: Options) -> RootMachine {
    RootMachine::new("vi", Box::new(Machine::new(options, Some(Box::new(Words)))))
}

fn key_event(key: char) -> InputEvent {
    InputEvent {
        kind: InputType::Character,
        key: key.to_string(),
        timestamp: 0,
    }
}

fn command_event(kind: InputType) -> InputEvent {
    InputEvent {
        kind,
        key: String::new(),
        timestamp: 0,
    }
}

fn named_event(kind: InputType, key: &str) -> InputEvent {
    InputEvent {
        kind,
        key: key.to_owned(),
        timestamp: 0,
    }
}

#[test]
fn regression_matrix() {
    let telex = options("telex", false, true);
    let simple = options("telex", true, true);
    let cases = [
        ("toans", "toán", simple.clone()),
        ("toanfs", "toán", simple.clone()),
        ("toansz", "toan", simple.clone()),
        ("aa", "â", simple.clone()),
        ("aw", "ă", simple.clone()),
        ("ee", "ê", simple.clone()),
        ("oo", "ô", simple.clone()),
        ("ow", "ơ", simple.clone()),
        ("uw", "ư", simple.clone()),
        ("as", "á", simple.clone()),
        ("af", "à", simple.clone()),
        ("ar", "ả", simple.clone()),
        ("ax", "ã", simple.clone()),
        ("aj", "ạ", simple.clone()),
        ("Huynhf", "Huỳnh", simple.clone()),
        ("huyeenf", "huyền", simple.clone()),
        ("nguowif", "người", simple.clone()),
        ("ddieeuf", "điều", simple.clone()),
        ("tieengs", "tiếng", telex.clone()),
        ("dduwowngf", "đường", telex.clone()),
        ("nuawx", "nữa", telex.clone()),
        ("nuwax", "nữa", telex.clone()),
        ("uow", "ươ", telex.clone()),
        ("UOW", "ƯƠ", telex.clone()),
        ("uow", "ươ", simple.clone()),
        ("ddaya", "đây", telex.clone()),
        ("daya", "dây", telex.clone()),
        ("caya", "cây", telex.clone()),
        ("duowjcd", "được", telex.clone()),
        ("nhieue", "nhiêu", telex.clone()),
        ("chueyern", "chuyển", telex.clone()),
        ("dduocwj", "được", telex.clone()),
        ("dduwocj", "được", telex.clone()),
        ("software", "software", telex.clone()),
        ("network", "network", telex.clone()),
        ("deterministic", "deterministic", telex.clone()),
        ("password", "password", telex.clone()),
        ("coffee", "coffee", telex.clone()),
        ("tesst", "test", telex),
    ];
    for (input, expected, opts) in cases {
        assert_eq!(run(input, opts), expected, "{input}");
    }
}

#[test]
fn explicit_double_cancel_at_token_end() {
    let opts = options("telex", false, true);
    let cases = [
        ("pass", "pas"),
        ("PASS", "PAS"),
        ("passs", "pass"),
        ("PASSS", "PASS"),
        ("ass", "as"),
        ("asss", "ass"),
        ("exx", "ex"),
        ("exxe", "exe"),
    ];
    for (input, expected) in cases {
        assert_eq!(run(input, opts.clone()), expected, "{input}");
    }
}

#[test]
fn explicit_cancel_is_committed_before_english_recovery() {
    let mut machine = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    let mut got = String::new();
    for key in "pass".chars() {
        got = machine.character(key);
    }
    assert_eq!(got, "pas");
    let state = machine.state();
    assert!(!state.ambiguous);
    assert_eq!(state.mode, Mode::RawLocked);
    assert_eq!(state.phase, Phase::Dead);
    assert_eq!(state.fallback, "pas");

    assert_eq!(machine.character('w'), "pasw");
    for key in "ord".chars() {
        got = machine.character(key);
    }
    assert_eq!(got, "password");
}

#[test]
fn cancel_returns_to_raw_typing_state() {
    let cases = [
        ("assk", "ask"),
        ("affter", "after"),
        ("urrl", "url"),
        ("exxe", "exe"),
        ("ajjax", "ajax"),
    ];
    for simple in [false, true] {
        let opts = options("telex", simple, true);
        for (input, expected) in cases {
            assert_eq!(
                run(input, opts.clone()),
                expected,
                "simple={simple} {input}"
            );
        }
    }

    let mut machine = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    for key in "urr".chars() {
        machine.character(key);
    }
    let state = machine.state();
    assert_eq!(state.raw, "urr");
    assert_eq!(state.fallback, "ur");
    assert_eq!(state.rendered, "ur");
    assert_eq!(state.mode, Mode::RawLocked);
    assert_eq!(state.phase, Phase::Dead);
    assert!(!state.ambiguous);
    assert_eq!(machine.character('l'), "url");
}

#[test]
fn cancel_preference_cannot_override_explicit_repeat() {
    let mut opts = options("telex", false, true);
    opts.cancel_preference = "english".to_owned();
    for (input, expected) in [
        ("pass", "pas"),
        ("passs", "pass"),
        ("password", "password"),
        ("tesst", "test"),
    ] {
        assert_eq!(run(input, opts.clone()), expected, "{input}");
    }
}

#[test]
fn bracket_keys_are_not_vietnamese_shortcuts() {
    let machine = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    assert!(!machine.accepts_character('['));
    assert!(!machine.accepts_character(']'));
}

#[test]
fn event_observation_does_not_drive_fsm() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let collector = EventCollector {
        events: Arc::clone(&events),
    };
    let mut session = Session::new(
        root(options("telex", false, true)),
        Some(Box::new(collector)),
    );

    for key in "ddaya".chars() {
        session.handle(key_event(key));
    }
    let output = session.handle(named_event(InputType::Lifecycle, "finalize"));
    assert_eq!(output.commit, "đây");

    let events = events.lock().expect("event lock");
    assert!(!events.is_empty());
    assert!(events
        .iter()
        .any(|event| event.kind == CoreEventType::TextCommitted && event.text == "đây"));
}

#[test]
fn replay_is_deterministic() {
    let mut inputs: Vec<InputEvent> = "nuawx".chars().map(key_event).collect();
    inputs.push(named_event(InputType::Lifecycle, "finalize"));

    let first = replay(root(options("telex", false, true)), &inputs, None);
    let second = replay(root(options("telex", false, true)), &inputs, None);
    assert_eq!(first.final_output.commit, "nữa");
    assert_eq!(first.final_output, second.final_output);
}

#[test]
fn late_shape_cancel() {
    let cases = [
        ("telex", "data", "dât", "data", 'a'),
        ("telex", "DATA", "DÂT", "DATA", 'A'),
        ("telex", "daya", "dây", "daya", 'a'),
        ("telex", "tene", "tên", "tene", 'e'),
        ("telex", "tono", "tôn", "tono", 'o'),
        ("telex", "banw", "băn", "banw", 'w'),
        ("telex", "duowjcd", "được", "duowjcd", 'd'),
        ("vni", "dan9", "đan", "dan9", '9'),
        ("telex", "aa", "â", "aa", 'a'),
        ("telex", "uow", "ươ", "uow", 'w'),
        ("vni", "a6", "â", "a6", '6'),
    ];
    for (method, raw, live, cancelled, key) in cases {
        for auto_restore in [false, true] {
            let mut machine =
                Machine::new(options(method, false, auto_restore), Some(Box::new(Words)));
            for input in raw.chars() {
                machine.character(input);
            }
            assert_eq!(machine.displayed(), live, "{raw} auto={auto_restore}");
            assert_eq!(machine.character(key), cancelled, "{raw} cancel");
            assert_eq!(
                machine.control(CompositionControl::Backspace),
                live,
                "{raw} undo before boundary"
            );
            assert_eq!(machine.character(key), cancelled, "{raw} cancel again");
            assert_eq!(
                machine.lifecycle(LifecycleEvent::Finalize),
                cancelled,
                "{raw} finalize"
            );
            assert!(
                !machine.active(),
                "{raw} lifecycle boundary ends composition"
            );

            let mut continued =
                Machine::new(options(method, false, auto_restore), Some(Box::new(Words)));
            for input in raw.chars() {
                continued.character(input);
            }
            assert_eq!(
                continued.character(key),
                cancelled,
                "{raw} continuation cancel"
            );
            assert_eq!(
                continued.character(key),
                format!("{cancelled}{key}"),
                "{raw} literal continuation"
            );
        }
    }
}

#[test]
fn data_finalize_and_keep_displayed() {
    let mut machine = Machine::new(options("telex", false, true), None);
    for key in "data".chars() {
        machine.character(key);
    }
    assert_eq!(machine.lifecycle(LifecycleEvent::Finalize), "data");

    machine.lifecycle(LifecycleEvent::Reset);
    for key in "data".chars() {
        machine.character(key);
    }
    let kept = machine.displayed().to_owned();
    machine.lifecycle(LifecycleEvent::Reset);

    assert_eq!(kept, "dât");
    assert_eq!(machine.lifecycle(LifecycleEvent::Finalize), "");
    assert!(!machine.active());
    assert_eq!(machine.control(CompositionControl::Backspace), "");
}

#[test]
fn closed_only_vowels_restore_raw_at_word_boundary() {
    for (raw, live) in [("thaas", "th\u{1ea5}"), ("taws", "t\u{1eaf}")] {
        let mut machine = Machine::new(options("telex", false, true), None);
        for key in raw.chars() {
            machine.character(key);
        }
        assert_eq!(machine.displayed(), live, "{raw} live");
        assert_eq!(
            machine.lifecycle(LifecycleEvent::Finalize),
            raw,
            "{raw} finalize"
        );
    }

    for (raw, committed) in [("thaats", "th\u{1ea5}t"), ("tawts", "t\u{1eaf}t")] {
        let mut machine = Machine::new(options("telex", false, true), None);
        for key in raw.chars() {
            machine.character(key);
        }
        assert_eq!(
            machine.lifecycle(LifecycleEvent::Finalize),
            committed,
            "{raw} valid closed syllable"
        );
    }

    for (raw, committed) in [("aa", "\u{00e2}"), ("aw", "\u{0103}")] {
        let mut machine = Machine::new(options("telex", false, true), None);
        for key in raw.chars() {
            machine.character(key);
        }
        assert_eq!(
            machine.lifecycle(LifecycleEvent::Finalize),
            committed,
            "{raw} standalone letter"
        );
    }
}

#[test]
fn repeat_cancel_cannot_be_disabled() {
    let cases = [
        ("telex", "docss", "docs"),
        ("telex", "DOCSS", "DOCS"),
        ("telex", "docsss", "docss"),
        ("telex", "DOCSSS", "DOCSS"),
        ("telex", "docssss", "docsss"),
        ("telex", "docsssabc", "docssabc"),
        ("telex", "off", "of"),
        ("telex", "carr", "car"),
        ("telex", "exx", "ex"),
        ("telex", "docjj", "docj"),
        ("telex", "dataa", "data"),
        ("telex", "pass", "pas"),
        ("telex", "passs", "pass"),
        ("telex", "password", "password"),
        ("telex", "coffee", "coffee"),
        ("telex", "tesst", "test"),
        ("vni", "doc11", "doc1"),
    ];

    for legacy in [false, true] {
        for preference in ["explicit", "english"] {
            for (method, input, expected) in cases {
                let mut opts = options(method, false, true);
                opts.double_cancel = legacy;
                opts.cancel_preference = preference.to_owned();
                assert_eq!(
                    run(input, opts),
                    expected,
                    "legacy={legacy} pref={preference} input={input}"
                );
            }
        }
    }
}

#[test]
fn raw_boundary_commits_physical_keys_and_ends_the_token() {
    let mut machine = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    for key in "ddc".chars() {
        machine.character(key);
    }
    assert_eq!(machine.displayed(), "đc");
    assert_eq!(machine.raw_boundary(), "ddc");
    assert_eq!(machine.state().mode, Mode::Start);
    assert_eq!(machine.physical(), "");
    assert!(!machine.active());

    let mut session = Session::new(root(options("telex", false, true)), None);
    for key in "ddc".chars() {
        session.handle(key_event(key));
    }
    let output = session.handle(command_event(InputType::RawBoundary));
    assert_eq!(output.commit, "ddc");
    assert_eq!(output.rendered, "");
}

#[test]
fn correction_is_deferred_to_root_boundary() {
    let mut machine = root(options("telex", false, true));
    for key in "chueyern".chars() {
        machine.character(key);
    }
    assert_ne!(machine.displayed(), "chuyển");
    assert_eq!(machine.space_boundary(), "chuyển ");
}

#[test]
fn correcting_boundary_adjusts_but_caret_move_preserves_visible_text() {
    let mut word_boundary = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    for key in "data".chars() {
        word_boundary.character(key);
    }
    assert_eq!(word_boundary.displayed(), "dât");
    assert_eq!(word_boundary.lifecycle(LifecycleEvent::Finalize), "data");
    assert_eq!(word_boundary.state().mode, Mode::Start);
    assert!(!word_boundary.active());

    let mut caret_boundary = root(options("telex", false, true));
    for key in "data".chars() {
        caret_boundary.character(key);
    }
    assert_eq!(caret_boundary.displayed(), "dât");
    assert_eq!(caret_boundary.caret_move(CaretMoveCause::Other), "dât");
    assert_eq!(caret_boundary.state().phase, RootPhase::Idle);
    assert!(!caret_boundary.active());

    let mut stroked_d = root(options("telex", false, true));
    stroked_d.character('d');
    stroked_d.character('d');
    assert_eq!(stroked_d.displayed(), "đ");
    assert_eq!(stroked_d.caret_move(CaretMoveCause::Other), "đ");
    assert_eq!(stroked_d.state().phase, RootPhase::Idle);
}

#[test]
fn smart_correction_preserves_base_order_and_only_floats_modifiers() {
    let opts = options("telex", false, true);

    for raw in ["mac", "macos", "mod"] {
        let mut machine = Machine::new(opts.clone(), Some(Box::new(Words)));
        for key in raw.chars() {
            machine.character(key);
        }
        assert_eq!(machine.displayed(), raw, "{raw} live");
        assert_eq!(
            machine.lifecycle(LifecycleEvent::Finalize),
            raw,
            "{raw} child finalize"
        );
        assert_eq!(run(raw, opts.clone()), raw, "{raw} root boundary");
    }

    let mut live = Machine::new(opts.clone(), Some(Box::new(Words)));
    for key in "thuongwf".chars() {
        live.character(key);
    }
    assert_ne!(live.displayed(), "thường");
    assert_ne!(live.lifecycle(LifecycleEvent::Finalize), "thường");

    for raw in ["thuongwf", "thuongfw", "thuowngf"] {
        assert_eq!(run(raw, opts.clone()), "thường", "{raw} root boundary");
    }
}

#[test]
fn oe_medial_rejects_labial_onsets_without_rejecting_valid_oe_syllables() {
    assert_eq!(run("khoer", options("telex", false, true)), "khỏe");
}

#[test]
fn speculative_invalid_telex_rolls_back_on_the_next_key() {
    let mut machine = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    assert_eq!(machine.character('m'), "m");
    assert_eq!(machine.character('o'), "mo");
    assert_eq!(machine.character('r'), "mỏ");
    assert_eq!(machine.character('e'), "mỏe");
    assert_eq!(machine.state().phase, Phase::PendingValidation);

    assert_eq!(machine.character('k'), "morek");
    assert_eq!(machine.state().mode, Mode::RawLocked);

    let mut space = root(options("telex", false, true));
    for key in "more".chars() {
        space.character(key);
    }
    assert_eq!(space.displayed(), "mỏe");
    assert_eq!(space.space_boundary(), "more ");
}

#[test]
fn boundary_correction_preserves_physical_prefix() {
    let opts = options("telex", false, true);

    for raw in ["stop", "start", "store"] {
        assert_eq!(run(raw, opts.clone()), raw, "physical onset: {raw}");

        let mut punctuation = root(opts.clone());
        for key in raw.chars() {
            punctuation.character(key);
        }
        assert_eq!(
            punctuation.punctuation_boundary('.'),
            format!("{raw}."),
            "punctuation onset: {raw}"
        );
    }

    for raw in ["as", "af", "ar", "ax", "aj"] {
        let corrected = run(raw, opts.clone());
        assert!(
            corrected.starts_with([
                'a', 'á', 'à', 'ả', 'ã', 'ạ', 'ă', 'ắ', 'ằ', 'ẳ', 'ẵ', 'ặ', 'â', 'ấ', 'ầ', 'ẩ',
                'ẫ', 'ậ'
            ]),
            "a-family escaped for {raw}: {corrected}"
        );
    }

    for (raw, family) in [
        ("es", "eéèẻẽẹêếềểễệ"),
        ("os", "oóòỏõọôốồổỗộơớờởỡợ"),
        ("us", "uúùủũụưứừửữự"),
        ("is", "iíìỉĩị"),
        ("ys", "yýỳỷỹỵ"),
    ] {
        let corrected = run(raw, opts.clone());
        let first = corrected.chars().next().expect("boundary output");
        assert!(
            family.contains(first),
            "initial vowel family changed for {raw}: {corrected}"
        );
    }
}

#[test]
fn space_and_punctuation_share_vietnamese_boundary_policy() {
    let expected = "\u{0111}\u{01b0}\u{1ee3}c";

    let mut space = root(options("telex", false, true));
    for key in "dduwocj".chars() {
        space.character(key);
    }
    assert_eq!(space.space_boundary(), format!("{expected} "));
    assert_eq!(space.state().phase, RootPhase::Idle);

    let mut punctuation = root(options("telex", false, true));
    for key in "dduwocj".chars() {
        punctuation.character(key);
    }
    assert_eq!(
        punctuation.punctuation_boundary('.'),
        format!("{expected}.")
    );
    assert_eq!(punctuation.state().phase, RootPhase::Idle);
}

#[test]
fn shortcut_boundary_commits_displayed_without_correction() {
    let mut machine = root(options("telex", false, true));
    for key in "data".chars() {
        machine.character(key);
    }
    let displayed = machine.displayed();
    assert_ne!(displayed, "data");
    assert_eq!(machine.shortcut_boundary(), displayed);
    assert_eq!(machine.state().phase, RootPhase::Idle);
    assert!(!machine.active());
}

#[test]
fn smart_correction_can_be_disabled() {
    let mut opts = options("telex", false, true);
    opts.smart_correction = false;
    assert_eq!(run("nhieue", opts), "nhieue");
}

#[test]
fn smart_correction_never_invents_an_untyped_tone() {
    let opts = options("telex", false, true);
    assert_eq!(run("dduwoc", opts.clone()), "dduwoc");
    assert_eq!(run("dduocw", opts), "dduocw");
}
