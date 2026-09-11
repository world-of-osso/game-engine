# Native Windows development

Ordinary Cargo builds enable the `dev` feature for Bevy dynamic linking. On Windows, use `cargo bw` to build and `cargo rw -- [args]` to run: they select the normal development profile, CASC, and `x86_64-pc-windows-gnu`; Unix socket IPC is excluded explicitly.

## Prerequisites

- Rust meeting `Cargo.toml`'s minimum version.
- `rustup target add x86_64-pc-windows-gnu`.
- A GNU Windows C/C++ toolchain (`gcc`, `g++`, `ar`) and CMake on PATH.
- Sibling repositories referenced by the manifest: asset-resolver, shared-protocol, ui-toolkit, ui-toolkit-macros.

Run through Cargo so the development DLL search paths are configured. Distribution builds omit `dev` explicitly: `cargo build --release --target x86_64-pc-windows-gnu --no-default-features --features casc --bin game-engine`.

## Compatibility evidence

Native desktop investigation, 2026-09-11:

- MSVC failed compiling bundled `libquickjs-sys 0.9.0` C sources (including `JSValue` casts). Do not select MSVC with the current QuickJS dependency.
- The isolated QuickJS dependency compiled successfully with the GNU target in 36.08 seconds on the desktop's existing GNU toolchain.
- Full native GNU engine build and launch remain unverified until the current build completes.

The dependency's [Windows support documentation](https://github.com/theduke/quickjs-rs#windows-support) specifies the GNU target. Changing JavaScript runtimes is outside this build-configuration change.

Linux linker flags remain under the Linux target table. Dependency optimization levels are unchanged. Runtime game assets are separate from source compilation; a build alone does not prove game-world loading.
