# Godot cold import crash (exit 134/139)

**Symptom.** A cold `godot --headless --import` (fresh checkout or worktree: no `godot/.godot/`,
run by the launcher's `import_project_once` and by `deploy.sh`) intermittently crashes:
exit 134 (SIGABRT from Godot's crash handler) or 139, with `malloc(): unaligned tcache chunk
detected`, `/root propagate_notification()` thread errors, or a crash at exit. Rate on the
pinned `4.7.2-pr123946`: 2/10, 4/20 without the extension, 3/20 on a symbolized build.

**Root cause.** Godot engine race, godotengine/godot#111039.
`ClassDB::class_get_default_property_value()` fills two static caches (`default_values`,
`default_values_cached`) on the first call per class without taking ClassDB's lock. The
`glsl` importer (`ResourceImporterShaderFile`, `can_import_threaded() == true`) imports the
project's `.glsl` files on WorkerThreadPool threads, and each `ResourceSaver::save` of the
`RDShaderFile` asks `PropertyUtils::get_property_default_value` for default values. On a cold
process two threads fill the cache for the same class at once and corrupt the heap; the
crash surfaces later at an unrelated site. Symbolized core (two workers in the same
`HashMap` at `class_db.cpp:2216`, called from `ResourceFormatSaverBinaryInstance::save`):
`data/diagnostics/coldimport-2026-10-03/sym-*.log`, cores via `coredumpctl`.

**Not the cause.** The Rust extension (crashes persist with `game_engine.gdextension`
removed, `noext-*.log`). The off-thread `ShaderFile` `is_visible_in_tree()` errors from the
two raw-GLSL test fixtures that fail to parse (crashes persist with those skipped,
`skip-*.log`). Hot-loading the extension during the first scan. With every `.glsl` import
skipped: 0/30 (`noglsl-*.log`).

**Fix.** Upstream PR #123546 (merged to master 2026-10-02) takes `Locker::Lock(STATE_WRITE)`
at the top of the function. `scripts/godot/pr123546-classdb-default-values-race.patch`
backports it; the pin is now `4.7.2-pr123946-pr123546`
([Wayland exit hang](godot-wayland-exit-hang.md) has the pin mechanics). Symbolized
4.7.2 + both patches: 0/40 cold imports (`fixed-*.log`).

**Retirement.** Drop the patch with the pin once an official Godot release contains #123546.
