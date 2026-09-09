fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winres::WindowsResource::new();
    resource.set_icon("assets/icon.ico");
    resource.set("ProductName", "NeoHtop");
    resource.set("FileDescription", "NeoHtop");
    resource.compile().expect("embed Windows application icon");
}
