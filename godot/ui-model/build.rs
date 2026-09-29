fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Shared root data modules drop their Bevy derives under `godot_host`.
    println!("cargo::rustc-check-cfg=cfg(godot_host)");
    println!("cargo::rustc-cfg=godot_host");
}
