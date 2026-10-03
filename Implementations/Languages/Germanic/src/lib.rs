//! InputKey-defined Telex-style language packs for Danish, Swedish, and German.
//!
//! Each pack uses short ASCII postfix pairs chosen for typing ergonomics and
//! memorability. These mappings are InputKey conventions, not language standards.

use inputkey_core_abstractions::{
    CompositionControl, LanguageConfig, LanguageMachinePort, LanguageMetadata,
    LanguageMethodMetadata, LanguagePackPort, LanguageState, LanguageTransitionResult,
    LifecycleEvent, RootInput, RootTransition,
};

#[derive(Clone, Copy)]
struct Rule {
    left: char,
    key: char,
    lower: &'static str,
    upper: &'static str,
}

#[derive(Clone)]
struct Snapshot {
    raw: String,
    rendered: String,
    literal: bool,
}

struct Machine {
    raw: String,
    rendered: String,
    literal: bool,
    history: Vec<Snapshot>,
    rules: &'static [Rule],
}

impl Machine {
    fn new(rules: &'static [Rule]) -> Self {
        Self {
            raw: String::new(),
            rendered: String::new(),
            literal: false,
            history: Vec::new(),
            rules,
        }
    }

    fn transform(&mut self, key: char) -> bool {
        let Some(last) = self.rendered.chars().last() else {
            return false;
        };
        let left = last.to_ascii_lowercase();
        let modifier = key.to_ascii_lowercase();
        let Some(rule) = self
            .rules
            .iter()
            .find(|rule| rule.left == left && rule.key == modifier)
        else {
            return false;
        };

        self.rendered.pop();
        self.rendered.push_str(if last.is_uppercase() {
            rule.upper
        } else {
            rule.lower
        });
        true
    }

    fn apply_character(&mut self, key: char) -> String {
        self.history.push(Snapshot {
            raw: self.raw.clone(),
            rendered: self.rendered.clone(),
            literal: self.literal,
        });
        self.raw.push(key);

        if self.literal || !self.transform(key) {
            self.rendered.push(key);
        }
        self.rendered.clone()
    }

    fn apply_backspace(&mut self) -> String {
        if let Some(snapshot) = self.history.pop() {
            self.raw = snapshot.raw;
            self.rendered = snapshot.rendered;
            self.literal = snapshot.literal;
        }
        self.rendered.clone()
    }

    fn apply_escape(&mut self) -> String {
        self.literal = true;
        self.rendered = self.raw.clone();
        self.rendered.clone()
    }

    fn clear(&mut self) {
        self.raw.clear();
        self.rendered.clear();
        self.literal = false;
        self.history.clear();
    }
}

impl LanguageMachinePort for Machine {
    fn accepts_character(&self, character: char) -> bool {
        character.is_ascii_alphabetic() || (character == '\'' && !self.raw.is_empty())
    }

    fn on_transition(&mut self, transition: RootTransition) -> LanguageTransitionResult {
        let text = match transition.input {
            RootInput::Character(character) => self.apply_character(character),
            RootInput::CompositionControl(CompositionControl::Backspace) => self.apply_backspace(),
            RootInput::CompositionControl(CompositionControl::Escape) => self.apply_escape(),
            RootInput::SpaceBoundary | RootInput::PunctuationBoundary(_) => {
                let text = self.rendered.clone();
                self.clear();
                text
            }
            RootInput::CaretMoveBoundary(_) | RootInput::ShortcutBoundary => {
                let text = self.rendered.clone();
                self.clear();
                text
            }
            RootInput::RawBoundary => {
                let text = self.raw.clone();
                self.clear();
                text
            }
            RootInput::Lifecycle(LifecycleEvent::Finalize | LifecycleEvent::FocusLost) => {
                let text = self.rendered.clone();
                self.clear();
                text
            }
            RootInput::Lifecycle(_) => {
                self.clear();
                String::new()
            }
        };
        LanguageTransitionResult { text }
    }

    fn state(&self) -> LanguageState {
        LanguageState {
            raw: self.raw.clone(),
            rendered: self.rendered.clone(),
            mode: if self.literal { "literal" } else { "compose" }.into(),
            phase: if self.raw.is_empty() { "start" } else { "text" }.into(),
        }
    }
}

const DANISH_RULES: &[Rule] = &[
    Rule {
        left: 'a',
        key: 'e',
        lower: "æ",
        upper: "Æ",
    },
    Rule {
        left: 'o',
        key: 'e',
        lower: "ø",
        upper: "Ø",
    },
    Rule {
        left: 'a',
        key: 'w',
        lower: "å",
        upper: "Å",
    },
];

const SWEDISH_RULES: &[Rule] = &[
    Rule {
        left: 'a',
        key: 'e',
        lower: "ä",
        upper: "Ä",
    },
    Rule {
        left: 'o',
        key: 'e',
        lower: "ö",
        upper: "Ö",
    },
    Rule {
        left: 'a',
        key: 'w',
        lower: "å",
        upper: "Å",
    },
];

const GERMAN_RULES: &[Rule] = &[
    Rule {
        left: 'a',
        key: 'w',
        lower: "ä",
        upper: "Ä",
    },
    Rule {
        left: 'o',
        key: 'w',
        lower: "ö",
        upper: "Ö",
    },
    Rule {
        left: 'u',
        key: 'w',
        lower: "ü",
        upper: "Ü",
    },
    Rule {
        left: 's',
        key: 'z',
        lower: "ß",
        upper: "ẞ",
    },
];

fn metadata(id: &str, display_name: &str, native_name: &str, label: &str) -> LanguageMetadata {
    LanguageMetadata {
        id: id.into(),
        display_name: display_name.into(),
        native_name: native_name.into(),
        default_method: "telex".into(),
        methods: vec![LanguageMethodMetadata {
            id: "telex".into(),
            label: label.into(),
        }],
        options: Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DanishPack;

impl LanguagePackPort for DanishPack {
    fn metadata(&self) -> LanguageMetadata {
        metadata("da", "Danish", "Dansk", "InputKey Danish Telex")
    }

    fn create(&self, _config: LanguageConfig) -> Result<Box<dyn LanguageMachinePort>, String> {
        Ok(Box::new(Machine::new(DANISH_RULES)))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SwedishPack;

impl LanguagePackPort for SwedishPack {
    fn metadata(&self) -> LanguageMetadata {
        metadata("sv", "Swedish", "Svenska", "InputKey Swedish Telex")
    }

    fn create(&self, _config: LanguageConfig) -> Result<Box<dyn LanguageMachinePort>, String> {
        Ok(Box::new(Machine::new(SWEDISH_RULES)))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GermanPack;

impl LanguagePackPort for GermanPack {
    fn metadata(&self) -> LanguageMetadata {
        metadata("de", "German", "Deutsch", "InputKey German Telex")
    }

    fn create(&self, _config: LanguageConfig) -> Result<Box<dyn LanguageMachinePort>, String> {
        Ok(Box::new(Machine::new(GERMAN_RULES)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(pack: &dyn LanguagePackPort, raw: &str) -> String {
        let mut machine = pack.create(LanguageConfig::new("telex")).unwrap();
        let mut rendered = String::new();
        for key in raw.chars() {
            rendered = machine
                .on_transition(RootTransition {
                    from: inputkey_core_abstractions::RootPhase::Composing,
                    to: inputkey_core_abstractions::RootPhase::Composing,
                    input: RootInput::Character(key),
                })
                .text;
        }
        rendered
    }

    #[test]
    fn danish_telex_covers_letters_without_aa_collision() {
        assert_eq!(type_text(&DanishPack, "ae"), "æ");
        assert_eq!(type_text(&DanishPack, "oe"), "ø");
        assert_eq!(type_text(&DanishPack, "aw"), "å");
        assert_eq!(type_text(&DanishPack, "Koebenhavn"), "København");
        assert_eq!(type_text(&DanishPack, "Aarhus"), "Aarhus");
    }

    #[test]
    fn swedish_telex_covers_letters() {
        assert_eq!(type_text(&SwedishPack, "ae"), "ä");
        assert_eq!(type_text(&SwedishPack, "oe"), "ö");
        assert_eq!(type_text(&SwedishPack, "aw"), "å");
        assert_eq!(type_text(&SwedishPack, "saw"), "så");
    }

    #[test]
    fn german_telex_uses_w_for_umlauts_and_sz_for_eszett() {
        assert_eq!(type_text(&GermanPack, "aw"), "ä");
        assert_eq!(type_text(&GermanPack, "ow"), "ö");
        assert_eq!(type_text(&GermanPack, "uw"), "ü");
        assert_eq!(type_text(&GermanPack, "sz"), "ß");
        assert_eq!(type_text(&GermanPack, "strasze"), "straße");
    }

    #[test]
    fn modifier_case_never_changes_base_letter_case() {
        assert_eq!(type_text(&DanishPack, "Ae"), "Æ");
        assert_eq!(type_text(&DanishPack, "aE"), "æ");
        assert_eq!(type_text(&SwedishPack, "Oe"), "Ö");
        assert_eq!(type_text(&SwedishPack, "oE"), "ö");
        assert_eq!(type_text(&GermanPack, "Sz"), "ẞ");
        assert_eq!(type_text(&GermanPack, "sZ"), "ß");
    }
}
