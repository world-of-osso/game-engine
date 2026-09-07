# Compile latency

The development feature originally depended on `bevy_dylib` directly, which did not activate Bevy's dynamic-link import. `8fca26b9` changes `dev` to `bevy/dynamic_linking`; mold and incremental compilation are unchanged.

## Measured result

A same observable edit to the `--empty-window` error literal was built after a cache warmup:

| Build | Wall | Cargo |
| --- | ---: | ---: |
| Default at `f15e5050` | 21.173815 s | 20.98 s |
| Dynamic-link at `50991d70` | 6.956593 s | 6.86 s |
| Restored literal, dynamic-link | 4.566569 s | 4.50 s |

The 937.745784 s dynamic-link warmup and the 50.10 s default warmup followed deleted incremental caches, so neither is edit-build evidence. Other-project compilation was active during samples. These are not idle statistical results, general gameplay coverage, or proof of the requested under-three-second target; the target remains unmet.

## History

`5590b52f` (March 9, 2026) added the direct optional dependency for faster links. `f49f7ab5` later gated IPC for Windows compatibility but retained `dev = ["bevy_dylib"]`; no repository evidence says Windows disabled Bevy dynamic linking.

## Commands

Use the dynamic-link development commands documented in [AGENTS.md](../../../AGENTS.md). Distributed builds must omit `dev`.

## Sources

- [Cargo.toml](../../../Cargo.toml) — current feature wiring.
- [AGENTS.md](../../../AGENTS.md) — development commands and dylib runtime condition.
- Local Bevy 0.19 source, `/home/osso/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bevy-0.19.0/src/lib.rs` — `dynamic_linking` imports `bevy_dylib`.
- `data/diagnostics/compile-latency-20260907/` — ignored timing artifacts.
- `5590b52f`, `f49f7ab5`, `8fca26b9`, `50991d70` — repository history and fixes.

## See Also

- [[movement-performance]] — measurement interpretation limits.
