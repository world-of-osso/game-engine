# Native Windows development

Requires Rust 1.95 or newer with the MSVC toolchain and its C/C++ build tools.

Run from the engine checkout with sibling path dependencies available:

```text
cargo bw
cargo rw -- --screen login
cargo test --bin game-engine --no-default-features --features casc,dev <test_filter>
```

`bw`/`rw` use the existing dev profile and Bevy dynamic linking. They explicitly exclude Unix socket IPC; the Windows engine has no IPC transport or IPC CLI. Run through Cargo so development DLL search paths are supplied.

Linux retains its target-specific clang/mold flags and Wayland features. Build artifacts use the checkout-relative `target/` directory on both platforms. Optimization levels are unchanged.

For a Windows distribution build, omit dynamic linking:

```text
cargo build --release --bin game-engine --no-default-features --features casc
```

Native compilation and runtime compatibility remain unverified until the Windows desktop build completes. These commands do not establish compatibility of all native dependencies.
