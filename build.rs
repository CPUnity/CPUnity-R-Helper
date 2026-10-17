fn main() {
    println!("cargo:rerun-if-changed=razer-gui.rc");
    println!("cargo:rerun-if-changed=rhelper.ico");

    // Bake assets/logo.png and assets/banner.png into the exe when they exist.
    println!("cargo:rerun-if-changed=assets");
    for name in ["logo", "banner"] {
        println!("cargo:rustc-check-cfg=cfg(embedded_{name})");
        if std::path::Path::new(&format!("assets/{name}.png")).is_file() {
            println!("cargo:rustc-cfg=embedded_{name}");
        }
    }

    if cfg!(target_os = "windows") {
        // embed-resource 3.x: mark manifest as optional and unwrap the result so failures show clearly
        if let Err(e) =
            embed_resource::compile("razer-gui.rc", embed_resource::NONE).manifest_optional()
        {
            eprintln!("embed-resource failed: {e}");
        }
    }
}
