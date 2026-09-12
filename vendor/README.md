# Local dependency patches

## KTX MinGW environment prefix

`ktx2-rw/` copies crates.io `ktx2-rw` 0.2.4 (upstream commit `1dfa70d893ab7a393b5c3754a5e15c3a6d182453`) from the local Cargo registry. Package manifests, README, sources, build script, and manifest-declared examples are retained; registry metadata and the standalone lockfile are excluded. The published package declares `MIT OR Apache-2.0` and contains no separate license files; its license declaration, authorship, and source notices are preserved.

The build-script patch adds `lib/gcc/<triple>/<version>` and `<triple>/lib` search directories beneath an environment-supplied MinGW prefix. On the Windows desktop, `MINGW_PREFIX/lib` alone omitted WinLibs' GCC runtime archives and the target sysroot's `libwinpthread.a`. No compiler selection, RUSTFLAGS, or upstream alternate paths are changed. Retire this override when upstream discovers these directories for environment prefixes and the native GNU Windows engine links successfully. Native Windows linking is the verification boundary; no source-shape tests are added.

The vendored manifest uses bindgen 0.73.1. Bindgen 0.72 generated deprecated bitfield transmute/unsafe code under current Rust; the released update generates direct casts instead. Keep this dependency-level correction rather than post-processing `$OUT_DIR/bindings.rs` or suppressing generated warnings. Retire the local crate patch when upstream `ktx2-rw` incorporates both the MinGW search-path and modern bindgen changes.

## Empty mesh uploads

`bevy_render/` copies crates.io `bevy_render` 0.19.0 from the local registry, preserving its MIT/Apache licenses and package features. Registry metadata and the standalone lockfile are excluded. Workspace membership enables a real GPU allocator regression; the explicit dev-profile override preserves dependency optimization level 2.

The mesh allocator skips allocation for zero-byte vertex buffers but previously copied those meshes anyway, producing unallocated-key errors for vertex and index data. The copy loop now skips exactly the same empty meshes. Genuine missing-key diagnostics remain unchanged. This backports the empty-mesh correction described by Bevy issue #24874 and PR #24960; remove the override when the selected upstream release contains the fix and passes the regression.

`empty_mesh_upload_and_valid_empty_valid_lifecycle` uses a real GPU device, mixes empty and populated meshes, checks valid → empty → valid allocation lifetime, reads back vertex/index bytes, and rejects captured allocator errors. No importer filtering or version bump is included.

## Skin palettes

`bevy_pbr/` imports the unchanged crates.io `bevy_pbr` 0.19.0 sources and dual MIT/Apache licenses. Registry cache metadata and its standalone lockfile are excluded. It is patched locally and included in the workspace to test the real private skin allocation/extraction boundary using the root build configuration. An explicit dev-profile override retains its former dependency optimization level2; workspace membership must not turn the PBR comparison into an optimization-level comparison.

Prototype purpose: share identical ordered-joint/inverse-bindpose palettes across render batches, without removing mesh/joint entities or changing shader/batching contracts. At import there is no behavior change or performance claim. Retire the override if the prototype lacks behavioral/performance evidence, or an upstream release provides equivalent sharing with passing lifetime, motion-vector, and workload checks.

## Transform queue

`bevy_transform/` is the unmodified crates.io `bevy_transform` 0.19.0 source at import, retaining its licenses. Its explicit dev-profile override retains optimization level2: the dependency wildcard excludes workspace members. The earlier import omitted this override; Cargo artifact metadata confirmed level0 before correction. `Cargo.toml` patches that exact package locally and includes it as a workspace member so its real contention/transform tests use the root build profile and target cache. Only the crates listed here are locally patched.

Purpose: stop the actual empty-queue/active-worker spin. The current full-client sample attributes 19.46% of samples to transform-worker polling; 90.05% of that symbol lands on its lock-retry branch. Evidence: `data/diagnostics/cpu-goal-resumed/current-baseline/transform-hot-instructions-correct-symbol.log`.

Workers wait on a condition variable while the queue is empty and producers remain active. Pending batches and the active-producer count share one mutex, so completion is checked against actual work rather than queued wake tokens. Checked-out batches retire through a drop guard, including during unwinding. Worker count, hierarchy traversal, and batch ownership remain unchanged; no frame cap, production sleep, or worker limit is added.

The earlier empty-wake-token attempt could skip a later propagation pass: the regression expected child translation 23 but retained 13. `completion_wakes_do_not_skip_next_propagation_pass` preserves that boundary. `empty_work_queue_waits_for_busy_worker_without_spinning` requires at most two CPU ticks during 300 ms waiting; the original spin used 29.

Native CPU comparison is required; this patch is not performance proof. Retire the override when an upstream Bevy release removes the measured empty-queue spin and passes the queue wait, transform-lifecycle, and native CPU checks for this workload.
