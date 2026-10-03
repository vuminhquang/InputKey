//! Vietnamese language machine for InputKey.

mod machine;

use inputkey_core_abstractions::{
    LanguageConfig, LanguageMachinePort, LanguageMetadata, LanguageMethodMetadata,
    LanguageOptionMetadata, LanguagePackPort, LexiconPort,
};
use std::sync::Arc;

pub use machine::{process, Machine, Mode, Options, Phase, Tone, VietnameseState};

pub type LexiconFactory = Arc<dyn Fn() -> Option<Box<dyn LexiconPort>> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct VietnamesePack {
    lexicon_factory: LexiconFactory,
}

impl VietnamesePack {
    pub fn new(lexicon_factory: LexiconFactory) -> Self {
        Self { lexicon_factory }
    }

    pub fn without_lexicon() -> Self {
        Self::new(Arc::new(|| None))
    }
}

impl LanguagePackPort for VietnamesePack {
    fn metadata(&self) -> LanguageMetadata {
        LanguageMetadata {
            id: "vi".into(),
            display_name: "Vietnamese".into(),
            native_name: "Tiếng Việt".into(),
            default_method: "telex".into(),
            methods: vec![
                LanguageMethodMetadata {
                    id: "telex".into(),
                    label: "Telex".into(),
                },
                LanguageMethodMetadata {
                    id: "vni".into(),
                    label: "VNI".into(),
                },
            ],
            options: vec![
                LanguageOptionMetadata {
                    id: "simple_telex".into(),
                    label: "Simple Telex".into(),
                    default_enabled: false,
                },
                LanguageOptionMetadata {
                    id: "auto_restore".into(),
                    label: "Restore original keys automatically (Auto Restore)".into(),
                    default_enabled: true,
                },
                LanguageOptionMetadata {
                    id: "smart_correction".into(),
                    label: "Smart correction".into(),
                    default_enabled: true,
                },
            ],
        }
    }

    fn create(&self, config: LanguageConfig) -> Result<Box<dyn LanguageMachinePort>, String> {
        let method = if config.method.eq_ignore_ascii_case("vni") {
            "vni"
        } else {
            "telex"
        };
        let options = Options {
            method: method.into(),
            simple_telex: config.enabled("simple_telex", false),
            auto_restore: config.enabled("auto_restore", true),
            smart_correction: config.enabled("smart_correction", true),
            double_cancel: true,
            cancel_preference: config.value_or("cancel_preference", "explicit").to_owned(),
        };
        Ok(Box::new(Machine::new(options, (self.lexicon_factory)())))
    }
}
