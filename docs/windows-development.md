# Native Windows development

Ordinary Cargo builds enable the `dev` feature for Bevy dynamic linking. On Windows, use `./scripts/windows-dev.ps1 build` to build and `./scripts/windows-dev.ps1 run -- [args]` to run. The script selects the normal development profile, CASC, and `x86_64-pc-windows-gnu`; Unix socket IPC is excluded explicitly.

## Prerequisites

- Rust meeting `Cargo.toml`'s minimum version.
- `rustup target add x86_64-pc-windows-gnu`.
- A GNU Windows C/C++ toolchain (`gcc`, `g++`, `ar`) and CMake on PATH.
- LLVM in its standard Program Files location (`winget install --exact --id LLVM.LLVM`), required by KTX bindgen.
- Sibling repositories referenced by the manifest: asset-resolver, shared-protocol, ui-toolkit, ui-toolkit-macros.

`scripts/windows-dev.ps1` discovers the GNU compiler on PATH and sets process-local `MINGW_PREFIX` for ktx2-rw's library lookup. It also supplies LLVM's resource directory and the GNU sysroot to bindgen. This supports nonstandard GNU toolchain locations without machine-wide environment changes.

Run through Cargo so the development DLL search paths are configured. Distribution builds omit `dev` explicitly: `cargo build --release --target x86_64-pc-windows-gnu --no-default-features --features casc --bin game-engine`.

## Compatibility evidence

Native desktop investigation, 2026-09-11:

- MSVC failed compiling bundled `libquickjs-sys 0.9.0` C sources (including `JSValue` casts). Do not select MSVC with the current QuickJS dependency.
- The isolated QuickJS dependency compiled successfully with the GNU target in 36.08 seconds on the desktop's existing GNU toolchain.

### Build and launch

- Native GNU development build and the portable-login source build passed; actual engine `--help` launch passed through the PowerShell script. Bevy DLL loading is exercised by that launch.
- After creating an unrelated diagnostic log, a warm script `run -- --help` completed in 2.562 seconds without changing the executable timestamp. `build.rs` declares only its own source as input; log writes no longer force recompilation.
- Setting `GAME_ENGINE_MAX_MEM_GB=1` produced the expected explicit rejection and exit code 2.
- Interactive rendering/gameplay is not proven. White-window captures were inconclusive; no visual claim is made, per the September 11, 2026 decision to stop visual testing.
- The no-IPC build reports 107 library warnings, one engine-binary unused import, and a shared-protocol unused import. These were not suppressed.

`build.rs` supplies QuickJS's omitted GNU `winpthread` link dependency after native archives. Remove this directive when the QuickJS dependency itself declares and correctly orders that runtime link.

The dependency's [Windows support documentation](https://github.com/theduke/quickjs-rs#windows-support) specifies the GNU target. Changing JavaScript runtimes is outside this build-configuration change.

`GAME_ENGINE_MAX_MEM_GB` uses Unix RLIMIT_AS and is unsupported on Windows; setting it there fails explicitly rather than silently dropping a requested limit.

## Portable UI assets

The engine configures `ui_toolkit::font_registry::FontRegistry::with_directory("data/fonts")` before `UiPlugin`. The configured directory is authoritative: missing or invalid fonts fail at that path; the toolkit's existing default directory remains for consumers that do not configure one.

Local CASC supplies `data/fonts/FRIZQT__.TTF` (FDID 615960) and `data/fonts/ARIALN.ttf` (FDID 615958). `data/ui/` contains the Common-Input-Border BLP parts (FDIDs 374201–374209) used by login and character UI. Runtime-generated login button KTX2 files remain under `data/ui/`.

Linux linker flags remain under the Linux target table. Dependency optimization levels are unchanged. Runtime game assets are separate from source compilation; a build alone does not prove game-world loading.
