fn main() {
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let gnu = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
    if windows && gnu {
        // Bundled QuickJS uses pthread atomics but omits this Windows link dependency.
        // Put it after native archives so GNU ld resolves QuickJS's references.
        println!("cargo:rustc-link-arg=-lwinpthread");
    }
}
