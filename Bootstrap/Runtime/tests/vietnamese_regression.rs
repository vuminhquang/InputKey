use std::sync::{Arc, Mutex};

use inputkey_boundary::{InputEvent, InputType};
use inputkey_core_abstractions::{CoreEvent, CoreEventType, EventPublisher, LexiconPort};
use inputkey_language_vietnamese::{Machine, Mode, Options, Phase};
use inputkey_operators::{replay, Machine as RootMachine, Session};

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
        machine.type_key(key);
    }
    machine
        .decision_boundary(' ')
        .strip_suffix(' ')
        .unwrap_or_default()
        .to_owned()
}

fn root(options: Options) -> RootMachine {
    RootMachine::new("vi", Box::new(Machine::new(options, Some(Box::new(Words)))))
}

fn key_event(key: char) -> InputEvent {
    InputEvent {
        kind: InputType::Key,
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
        got = machine.type_key(key);
    }
    assert_eq!(got, "pas");
    let state = machine.state();
    assert!(!state.ambiguous);
    assert_eq!(state.mode, Mode::RawLocked);
    assert_eq!(state.phase, Phase::Dead);
    assert_eq!(state.fallback, "pas");

    assert_eq!(machine.type_key('w'), "pasw");
    for key in "ord".chars() {
        got = machine.type_key(key);
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
        machine.type_key(key);
    }
    let state = machine.state();
    assert_eq!(state.raw, "urr");
    assert_eq!(state.fallback, "ur");
    assert_eq!(state.rendered, "ur");
    assert_eq!(state.mode, Mode::RawLocked);
    assert_eq!(state.phase, Phase::Dead);
    assert!(!state.ambiguous);
    assert_eq!(machine.type_key('l'), "url");
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
fn bracket_shortcut_repeat_cancel() {
    for (key, shaped, cancelled) in [('[', "ư", "["), (']', "ơ", "]")] {
        for auto_restore in [false, true] {
            let mut opts = options("telex", false, auto_restore);
            opts.double_cancel = false;
            opts.cancel_preference = "english".to_owned();
            let mut machine = Machine::new(opts, Some(Box::new(Words)));

            assert_eq!(machine.type_key(key), shaped);
            assert_eq!(machine.type_key(key), cancelled);
            assert_eq!(machine.finalize(), cancelled);
            assert_eq!(machine.backspace(), shaped);

            machine.type_key(key);
            assert_eq!(machine.type_key(key), format!("{cancelled}{key}"));
        }
    }
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
    let output = session.handle(command_event(InputType::Finalize));
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
    inputs.push(command_event(InputType::Finalize));

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
                machine.type_key(input);
            }
            assert_eq!(machine.rendered_text(), live, "{raw} auto={auto_restore}");
            assert_eq!(machine.type_key(key), cancelled, "{raw} cancel");
            assert_eq!(machine.finalize(), cancelled, "{raw} finalize");
            assert_eq!(machine.backspace(), live, "{raw} undo");
            machine.type_key(key);
            assert_eq!(
                machine.type_key(key),
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
        machine.type_key(key);
    }
    assert_eq!(machine.finalize(), "data");

    machine.reset();
    for key in "data".chars() {
        machine.type_key(key);
    }
    let kept = machine.rendered_text().to_owned();
    machine.reset();

    assert_eq!(kept, "dât");
    assert_eq!(machine.finalize(), "");
    assert!(!machine.has_history());
    assert_eq!(machine.backspace(), "");
}

#[test]
fn closed_only_vowels_restore_raw_at_word_boundary() {
    for (raw, live) in [("thaas", "th\u{1ea5}"), ("taws", "t\u{1eaf}")] {
        let mut machine = Machine::new(options("telex", false, true), None);
        for key in raw.chars() {
            machine.type_key(key);
        }
        assert_eq!(machine.rendered_text(), live, "{raw} live");
        assert_eq!(machine.finalize(), raw, "{raw} finalize");
    }

    for (raw, committed) in [("thaats", "th\u{1ea5}t"), ("tawts", "t\u{1eaf}t")] {
        let mut machine = Machine::new(options("telex", false, true), None);
        for key in raw.chars() {
            machine.type_key(key);
        }
        assert_eq!(machine.finalize(), committed, "{raw} valid closed syllable");
    }

    for (raw, committed) in [("aa", "\u{00e2}"), ("aw", "\u{0103}")] {
        let mut machine = Machine::new(options("telex", false, true), None);
        for key in raw.chars() {
            machine.type_key(key);
        }
        assert_eq!(machine.finalize(), committed, "{raw} standalone letter");
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
        ("telex", "[[", "["),
        ("telex", "]]", "]"),
        ("telex", "[[[", "[["),
        ("telex", "]]]", "]]"),
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
        machine.type_key(key);
    }
    assert_eq!(machine.rendered_text(), "đc");
    assert_eq!(machine.commit_raw_boundary(), "ddc");
    assert_eq!(machine.state().mode, Mode::Start);
    assert_eq!(machine.raw_text(), "");
    assert!(!machine.has_history());

    let mut session = Session::new(root(options("telex", false, true)), None);
    for key in "ddc".chars() {
        session.handle(key_event(key));
    }
    let output = session.handle(command_event(InputType::CommitRawBoundary));
    assert_eq!(output.commit, "ddc");
    assert_eq!(output.rendered, "");
}

#[test]
fn correction_is_deferred_to_root_boundary() {
    let mut machine = root(options("telex", false, true));
    for key in "chueyern".chars() {
        machine.type_key(key);
    }
    assert_ne!(machine.rendered_text(), "chuyển");
    assert_eq!(machine.decision_boundary(' '), "chuyển ");
}

#[test]
fn word_boundary_adjusts_but_natural_ime_boundary_preserves_visible_text() {
    let mut word_boundary = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    for key in "data".chars() {
        word_boundary.type_key(key);
    }
    assert_eq!(word_boundary.rendered_text(), "dât");
    assert_eq!(word_boundary.commit_boundary(), "data");
    assert_eq!(word_boundary.state().mode, Mode::Start);
    assert!(!word_boundary.has_history());

    let mut natural_boundary = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    for key in "data".chars() {
        natural_boundary.type_key(key);
    }
    assert_eq!(natural_boundary.rendered_text(), "dât");
    assert_eq!(natural_boundary.commit_displayed(), "dât");
    let state = natural_boundary.state();
    assert_eq!(state.mode, Mode::Start);
    assert_eq!(state.phase, Phase::Start);
    assert_eq!(state.raw, "");
    assert_eq!(state.rendered, "");
    assert!(!natural_boundary.has_history());

    let mut stroked_d = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    stroked_d.type_key('d');
    stroked_d.type_key('d');
    assert_eq!(stroked_d.rendered_text(), "đ");
    assert_eq!(stroked_d.commit_displayed(), "đ");
    assert_eq!(stroked_d.state().mode, Mode::Start);
}

#[test]
fn smart_correction_preserves_base_order_and_only_floats_modifiers() {
    let opts = options("telex", false, true);

    for raw in ["mac", "macos", "mod"] {
        let mut machine = Machine::new(opts.clone(), Some(Box::new(Words)));
        for key in raw.chars() {
            machine.type_key(key);
        }
        assert_eq!(machine.rendered_text(), raw, "{raw} live");
        assert_eq!(machine.finalize(), raw, "{raw} child finalize");
        assert_eq!(run(raw, opts.clone()), raw, "{raw} root boundary");
    }

    let mut live = Machine::new(opts.clone(), Some(Box::new(Words)));
    for key in "thuongwf".chars() {
        live.type_key(key);
    }
    assert_ne!(live.rendered_text(), "thường");
    assert_ne!(live.finalize(), "thường");

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
    assert_eq!(machine.type_key('m'), "m");
    assert_eq!(machine.type_key('o'), "mo");
    assert_eq!(machine.type_key('r'), "mỏ");
    assert_eq!(machine.type_key('e'), "mỏe");
    assert_eq!(machine.state().phase, Phase::PendingValidation);

    assert_eq!(machine.type_key('k'), "morek");
    assert_eq!(machine.state().mode, Mode::RawLocked);

    let mut space = root(options("telex", false, true));
    for key in "more".chars() {
        space.type_key(key);
    }
    assert_eq!(space.rendered_text(), "mỏe");
    assert_eq!(space.decision_boundary(' '), "more ");
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
