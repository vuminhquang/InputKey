use std::sync::{Arc, Mutex};

use inputkey_boundary::{InputEvent, InputType, Options};
use inputkey_core_abstractions::{
    CoreEvent, CoreEventType, EventPublisher, LexiconPort, Mode, Phase, Tone,
};
use inputkey_operators::{replay, Machine, Session};

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
        double_cancel: true,
        cancel_preference: String::new(),
    }
}

fn run(raw: &str, options: Options) -> String {
    let mut machine = Machine::new(options, Some(Box::new(Words)));
    for key in raw.chars() {
        machine.type_key(key);
    }
    machine.finalize()
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
        options("telex", false, true),
        Some(Box::new(Words)),
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

    let first = replay(
        options("telex", false, true),
        &inputs,
        Some(Box::new(Words)),
        None,
    );
    let second = replay(
        options("telex", false, true),
        &inputs,
        Some(Box::new(Words)),
        None,
    );
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
fn literalize_token_restores_physical_raw_token() {
    let mut machine = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    for key in "refer".chars() {
        machine.type_key(key);
    }
    assert_eq!(machine.rendered_text(), "rể");

    assert_eq!(machine.literalize_token(), "refer");
    let state = machine.state();
    assert_eq!(state.raw, "refer");
    assert_eq!(state.fallback, "refer");
    assert_eq!(state.rendered, "refer");
    assert_eq!(state.mode, Mode::RawLocked);
    assert_eq!(state.phase, Phase::Dead);
    assert_eq!(state.tone, Tone::None);
    assert!(!state.ambiguous);
    assert!(!state.transformed);

    assert_eq!(machine.type_key('s'), "refers");
}

#[test]
fn literalize_token_is_undoable_and_available_through_session() {
    let mut machine = Machine::new(options("telex", false, true), Some(Box::new(Words)));
    for key in "refer".chars() {
        machine.type_key(key);
    }
    let before = machine.rendered_text().to_owned();
    assert_eq!(machine.literalize_token(), "refer");
    assert_eq!(machine.backspace(), before);

    let mut session = Session::new(options("telex", false, true), Some(Box::new(Words)), None);
    for key in "refer".chars() {
        session.handle(key_event(key));
    }
    let output = session.handle(command_event(InputType::LiteralizeToken));
    assert_eq!(output.rendered, "refer");
    assert!(output.changed);
    assert_eq!(output.mode, "raw_locked");
    assert_eq!(output.phase, "dead");
}
