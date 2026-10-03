//! French Telex language machine for InputKey.
//!
//! The method keeps the same postfix/doubling feel as Vietnamese Telex while
//! using French-specific transformations:
//! aa→â, ee→ê, ii→î, oo→ô, uu→û
//! es→é, af/ef/uf→à/è/ù
//! ex/ix/ux/yx→ë/ï/ü/ÿ
//! cc→ç, oe→œ, ae→æ

use inputkey_core_abstractions::{
    CompositionControl, LanguageConfig, LanguageMachinePort, LanguageMetadata,
    LanguageMethodMetadata, LanguagePackPort, LanguageState, LanguageTransitionResult,
    LifecycleEvent, RootInput, RootTransition,
};

#[derive(Clone)]
struct Snapshot {
    raw: String,
    rendered: String,
    literal: bool,
}

pub struct Machine {
    raw: String,
    rendered: String,
    literal: bool,
    history: Vec<Snapshot>,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            raw: String::new(),
            rendered: String::new(),
            literal: false,
            history: Vec::new(),
        }
    }

    fn pair(left: char, key: char) -> Option<&'static str> {
        Some(match (left, key) {
            ('a', 'a') => "â",
            ('A', 'A') | ('A', 'a') | ('a', 'A') => "Â",
            ('e', 'e') => "ê",
            ('E', 'E') | ('E', 'e') | ('e', 'E') => "Ê",
            ('i', 'i') => "î",
            ('I', 'I') | ('I', 'i') | ('i', 'I') => "Î",
            ('o', 'o') => "ô",
            ('O', 'O') | ('O', 'o') | ('o', 'O') => "Ô",
            ('u', 'u') => "û",
            ('U', 'U') | ('U', 'u') | ('u', 'U') => "Û",

            ('e', 's') => "é",
            ('E', 'S') | ('E', 's') | ('e', 'S') => "É",

            ('a', 'f') => "à",
            ('A', 'F') | ('A', 'f') | ('a', 'F') => "À",
            ('e', 'f') => "è",
            ('E', 'F') | ('E', 'f') | ('e', 'F') => "È",
            ('u', 'f') => "ù",
            ('U', 'F') | ('U', 'f') | ('u', 'F') => "Ù",

            ('e', 'x') => "ë",
            ('E', 'X') | ('E', 'x') | ('e', 'X') => "Ë",
            ('i', 'x') => "ï",
            ('I', 'X') | ('I', 'x') | ('i', 'X') => "Ï",
            ('u', 'x') => "ü",
            ('U', 'X') | ('U', 'x') | ('u', 'X') => "Ü",
            ('y', 'x') => "ÿ",
            ('Y', 'X') | ('Y', 'x') | ('y', 'X') => "Ÿ",

            ('c', 'c') => "ç",
            ('C', 'C') | ('C', 'c') | ('c', 'C') => "Ç",

            ('o', 'e') => "œ",
            ('O', 'E') | ('O', 'e') => "Œ",
            ('o', 'E') => "œ",
            ('a', 'e') => "æ",
            ('A', 'E') | ('A', 'e') => "Æ",
            ('a', 'E') => "æ",
            _ => return None,
        })
    }

    fn transform(&mut self, key: char) -> bool {
        let Some(last) = self.rendered.chars().last() else {
            return false;
        };
        let Some(replacement) = Self::pair(last, key) else {
            return false;
        };
        self.rendered.pop();
        self.rendered.push_str(replacement);
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

impl Default for Machine {
    fn default() -> Self {
        Self::new()
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

#[derive(Clone, Copy, Debug, Default)]
pub struct FrenchPack;

impl LanguagePackPort for FrenchPack {
    fn metadata(&self) -> LanguageMetadata {
        LanguageMetadata {
            id: "fr".into(),
            display_name: "French".into(),
            native_name: "Français".into(),
            default_method: "telex".into(),
            methods: vec![LanguageMethodMetadata {
                id: "telex".into(),
                label: "French Telex".into(),
            }],
            options: Vec::new(),
        }
    }

    fn create(&self, _config: LanguageConfig) -> Result<Box<dyn LanguageMachinePort>, String> {
        Ok(Box::new(Machine::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(raw: &str) -> String {
        let mut machine = Machine::new();
        for key in raw.chars() {
            machine.apply_character(key);
        }
        machine.rendered.clone()
    }

    #[test]
    fn french_telex_covers_accents_ligatures_and_cedilla() {
        for (raw, expected) in [
            ("ee", "ê"),
            ("es", "é"),
            ("ef", "è"),
            ("af", "à"),
            ("uf", "ù"),
            ("ex", "ë"),
            ("ix", "ï"),
            ("ux", "ü"),
            ("yx", "ÿ"),
            ("aa", "â"),
            ("ii", "î"),
            ("oo", "ô"),
            ("uu", "û"),
            ("cc", "ç"),
            ("oe", "œ"),
            ("ae", "æ"),
            ("garccon", "garçon"),
        ] {
            assert_eq!(type_text(raw), expected, "{raw}");
        }
    }

    #[test]
    fn backspace_restores_previous_telex_state() {
        let mut machine = Machine::new();
        machine.apply_character('e');
        assert_eq!(machine.apply_character('e'), "ê");
        assert_eq!(machine.apply_backspace(), "e");
    }

    #[test]
    fn raw_escape_keeps_physical_sequence() {
        let mut machine = Machine::new();
        machine.apply_character('e');
        machine.apply_character('s');
        assert_eq!(machine.state().rendered, "é");
        assert_eq!(machine.state().raw, "es");
        assert_eq!(machine.apply_escape(), "es");
    }

    #[test]
    fn apostrophes_are_literal_joiners() {
        assert_eq!(type_text("l'amour"), "l'amour");
        assert_eq!(type_text("qu'on"), "qu'on");
    }
}
