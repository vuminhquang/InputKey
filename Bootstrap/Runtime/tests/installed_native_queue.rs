#![cfg(windows)]

use inputkey_core_abstractions::RootInput;
use inputkey_runtime::{create_machine, default_config, Catalog};

#[test]
#[ignore = "requires INPUTKEY_LANGUAGE_PACK_DIR pointing to installed DLLs"]
fn installed_native_pack_keeps_known_english_raw_at_live_and_boundary() {
    let directory = std::env::var_os("INPUTKEY_LANGUAGE_PACK_DIR")
        .expect("set INPUTKEY_LANGUAGE_PACK_DIR to packaged runtime languages directory");
    assert!(std::path::Path::new(&directory).is_dir());
    let native = inputkey_language_pack_loader::discover();
    assert!(
        native.iter().any(|p| p.metadata().id == "vi"),
        "native Vietnamese DLL not loaded"
    );
    let mut catalog = Catalog::bundled();
    for pack in native {
        catalog.add_pack(pack);
    }
    let mut engine = create_machine(&catalog, "vi", default_config(&catalog, "vi")).unwrap();
    let mut prefix = String::new();
    for key in "queue".chars() {
        prefix.push(key);
        let text = engine.dispatch(RootInput::Character(key));
        eprintln!("key={key}, returned={text:?}, state={:?}", engine.state());
        assert_eq!(
            engine.state().raw,
            prefix,
            "native pack lost raw input after {key}"
        );
    }
    assert_eq!(engine.state().rendered, "queue");
    assert_eq!(engine.dispatch(RootInput::SpaceBoundary), "queue ");

    let mut interrupted = create_machine(&catalog, "vi", default_config(&catalog, "vi")).unwrap();
    for key in "queu".chars() {
        interrupted.dispatch(RootInput::Character(key));
    }
    eprintln!(
        "four-character boundary: {:?}",
        interrupted.dispatch(RootInput::SpaceBoundary)
    );
}
