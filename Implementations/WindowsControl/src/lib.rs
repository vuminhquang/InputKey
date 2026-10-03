use inputkey_core_abstractions::LanguageMetadata;
use std::sync::Arc;

#[derive(Clone)]
pub struct ControlActions {
    pub settings_changed: Arc<dyn Fn() + Send + Sync + 'static>,
    pub languages: Arc<dyn Fn() -> Vec<LanguageMetadata> + Send + Sync + 'static>,
    pub install_text_service: Arc<dyn Fn() + Send + Sync + 'static>,
    pub text_service_available: Arc<dyn Fn() -> bool + Send + Sync + 'static>,
    pub text_service_active: Arc<dyn Fn() -> bool + Send + Sync + 'static>,
}

#[cfg(windows)]
mod settings_window;
#[cfg(windows)]
mod startup;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
pub fn run(actions: ControlActions) {
    win32::run(actions);
}

#[cfg(not(windows))]
pub fn run(_actions: ControlActions) {}
