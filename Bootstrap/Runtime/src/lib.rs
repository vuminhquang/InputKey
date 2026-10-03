//! Composition of the root machine with installed language packs.

use inputkey_core_abstractions::{
    LanguageCatalogPort, LanguageConfig, LanguageMachinePort, LanguageMetadata, LanguagePackPort,
};
use inputkey_english_bloom::EnglishBloom;
use inputkey_language_french::FrenchPack;
use inputkey_language_germanic::{DanishPack, GermanPack, SwedishPack};
use inputkey_language_vietnamese::VietnamesePack;
use inputkey_operators::Machine;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct Catalog {
    packs: Vec<Arc<dyn LanguagePackPort>>,
}

impl Catalog {
    pub fn bundled() -> Self {
        let vietnamese = VietnamesePack::new(Arc::new(|| Some(Box::new(EnglishBloom::new()))));
        Self {
            packs: vec![
                Arc::new(vietnamese),
                Arc::new(FrenchPack),
                Arc::new(DanishPack),
                Arc::new(SwedishPack),
                Arc::new(GermanPack),
            ],
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn installed() -> Self {
        let mut catalog = Self::bundled();
        for pack in inputkey_language_pack_loader::discover() {
            catalog.add_pack(pack);
        }
        catalog
    }

    #[cfg(target_arch = "wasm32")]
    pub fn installed() -> Self {
        Self::bundled()
    }

    pub fn add_pack(&mut self, pack: Arc<dyn LanguagePackPort>) {
        let id = pack.metadata().id;
        self.packs.retain(|existing| existing.metadata().id != id);
        self.packs.push(pack);
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::bundled()
    }
}

impl LanguageCatalogPort for Catalog {
    fn languages(&self) -> Vec<LanguageMetadata> {
        self.packs.iter().map(|pack| pack.metadata()).collect()
    }

    fn create(
        &self,
        language_id: &str,
        config: LanguageConfig,
    ) -> Result<Box<dyn LanguageMachinePort>, String> {
        self.packs
            .iter()
            .find(|pack| pack.metadata().id == language_id)
            .ok_or_else(|| format!("language pack not found: {language_id}"))?
            .create(config)
    }
}

pub fn default_config(catalog: &dyn LanguageCatalogPort, language_id: &str) -> LanguageConfig {
    let metadata = catalog
        .languages()
        .into_iter()
        .find(|language| language.id == language_id);
    let Some(metadata) = metadata else {
        return LanguageConfig::new("");
    };
    let mut config = LanguageConfig::new(metadata.default_method);
    for option in metadata.options {
        config.toggles.insert(option.id, option.default_enabled);
    }
    config
}

pub fn config_from_json(
    catalog: &dyn LanguageCatalogPort,
    language_id: &str,
    method: Option<&str>,
    options_json: Option<&str>,
) -> LanguageConfig {
    let mut config = default_config(catalog, language_id);
    if let Some(method) = method.filter(|value| !value.is_empty()) {
        config.method = method.to_owned();
    }
    if let Some(text) = options_json.filter(|value| !value.is_empty()) {
        if let Ok(Value::Object(values)) = serde_json::from_str::<Value>(text) {
            for (key, value) in values {
                match value {
                    Value::Bool(enabled) => {
                        config.toggles.insert(key, enabled);
                    }
                    Value::String(value) => {
                        config.values.insert(key, value);
                    }
                    _ => {}
                }
            }
        }
    }
    config
}

pub fn create_machine(
    catalog: &dyn LanguageCatalogPort,
    language_id: &str,
    config: LanguageConfig,
) -> Result<Machine, String> {
    let child = catalog.create(language_id, config)?;
    Ok(Machine::new(language_id, child))
}

pub fn catalog_json(catalog: &dyn LanguageCatalogPort) -> String {
    let languages: Vec<Value> = catalog
        .languages()
        .into_iter()
        .map(|language| {
            json!({
                "id": language.id,
                "displayName": language.display_name,
                "nativeName": language.native_name,
                "defaultMethod": language.default_method,
                "methods": language.methods.into_iter().map(|method| json!({
                    "id": method.id,
                    "label": method.label
                })).collect::<Vec<_>>(),
                "options": language.options.into_iter().map(|option| json!({
                    "id": option.id,
                    "label": option.label,
                    "defaultEnabled": option.default_enabled
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    json!({ "languages": languages }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputkey_core_abstractions::{RootInput, TypingEnginePort};

    #[test]
    fn root_switches_language_without_knowing_children() {
        let catalog = Catalog::bundled();

        let mut vi = create_machine(&catalog, "vi", default_config(&catalog, "vi")).unwrap();
        vi.dispatch(RootInput::Character('d'));
        vi.dispatch(RootInput::Character('d'));
        assert_eq!(vi.dispatch(RootInput::SpaceBoundary), "đ ");

        let mut fr = create_machine(&catalog, "fr", default_config(&catalog, "fr")).unwrap();
        fr.dispatch(RootInput::Character('e'));
        fr.dispatch(RootInput::Character('s'));
        assert_eq!(fr.dispatch(RootInput::SpaceBoundary), "é ");

        for (language, raw, expected) in [
            ("da", "Koebenhavn", "København "),
            ("sv", "saw", "så "),
            ("de", "strasze", "straße "),
        ] {
            let mut machine =
                create_machine(&catalog, language, default_config(&catalog, language)).unwrap();
            for key in raw.chars() {
                machine.dispatch(RootInput::Character(key));
            }
            assert_eq!(machine.dispatch(RootInput::SpaceBoundary), expected);
        }
    }

    #[test]
    fn raw_boundary_is_identical_for_every_language() {
        let catalog = Catalog::bundled();
        for language in ["vi", "fr", "da", "sv", "de"] {
            let mut machine =
                create_machine(&catalog, language, default_config(&catalog, language)).unwrap();
            machine.dispatch(RootInput::Character('e'));
            if language == "fr" {
                machine.dispatch(RootInput::Character('s'));
            }
            let raw = machine.raw();
            assert_eq!(machine.dispatch(RootInput::RawBoundary), raw);
            assert!(!machine.history_active());
        }
    }
}
