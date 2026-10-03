use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let version = env::var("CARGO_PKG_VERSION").expect("package version");
    let mut resource = winres::WindowsResource::new();
    resource.set_icon("../../Implementations/WindowsHook/inputkey-v.ico");
    resource.set("ProductName", "InputKey");
    resource.set("FileVersion", &version);
    resource.set("ProductVersion", &version);
    resource.set("CompanyName", "RentAICoder");
    resource.set("FileDescription", "InputKey multilingual input method");
    resource.set("LegalCopyright", "Copyright (c) 2026 RentAICoder");
    resource.set("Comments", "Free and open-source multilingual input method");
    resource.compile().expect("compile Windows resources");
}
