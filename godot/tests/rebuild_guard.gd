## Expensive rebuilds per frame over a measured segment: the extension's
## `RebuildCounters` categories (rust/src/rebuild_guard.rs) plus shadow-atlas
## reallocations, and Godot's own allocation counters. A steady frame (static camera and
## world, live clock) must rebuild nothing beyond `ALLOWLIST`; `violations` names each
## category over budget with its first call site. Used by frame_benchmark.gd's
## `rebuilds_per_frame`, the Windows receipt harness and rebuild_guard_live.gd; builds
## without `RebuildCounters` report `rebuild_guard_error` instead.
extends RefCounted

## Half-minutes per lit-time step: core world_time LIGHT_STEP_HALF_MINUTES. With the live
## clock the world light re-samples and re-applies once per step (rust/src/lighting/mod.rs
## WorldLighting::sync). Steps are counted on the client's clock, not the wall: a stall
## before a segment (the harness's screenshot) applies a step that was due earlier inside it.
const LIGHT_STEP_HALF_MINUTES := 1.0 / 32.0
## Half-minutes in a day, where the clock wraps.
const DAY_HALF_MINUTES := 2880.0
## The work a steady frame does by design, each with its source: `per_frame` operations
## every frame, `per_light_step` operations per lit-time step in the segment.
const ALLOWLIST := {
	# M2MaterialClock.advance_time_ms (rust/src/assets/uv_animation.rs) sets the
	# `liquid_animation_time_ms` global once per frame: liquid UV animation reads it.
	# A light step sets 32 scene globals: TerrainLight::bind_scene's 5 light, 20 fog
	# (bind_fog) and 7 water uniforms.
	"shader_broadcast": {"per_frame": 1, "per_light_step": 32},
	# One light application per lit-time step.
	"environment": {"per_light_step": 1},
	# WorldLighting::sync rewrites the sky reflection cubemap only when that application
	# changes the dome's colour stops (rust/src/lighting/mod.rs update_cubemap).
	"sky_cubemap": {"per_light_step": 1},
}
## Godot reallocates a shadow atlas only when its size or a quadrant subdivision is set
## (servers/rendering/renderer_rd/storage_rd/light_storage.cpp shadow_atlas_set_size,
## shadow_atlas_set_quadrant_subdivision, directional_shadow_atlas_set_size); no client
## code sets them, so the root viewport's settings are sampled every frame.
const DIRECTIONAL_SIZE := "rendering/lights_and_shadows/directional_shadow/size"
const DIRECTIONAL_16_BITS := "rendering/lights_and_shadows/directional_shadow/16_bits"
const RENDERING_INFOS := {
	"pipeline_compilations_canvas": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_CANVAS,
	"pipeline_compilations_mesh": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_MESH,
	"pipeline_compilations_surface": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_SURFACE,
	"pipeline_compilations_draw": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_DRAW,
	"pipeline_compilations_specialization": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_SPECIALIZATION,
	"texture_mem_bytes": RenderingServer.RENDERING_INFO_TEXTURE_MEM_USED,
	"buffer_mem_bytes": RenderingServer.RENDERING_INFO_BUFFER_MEM_USED,
	"video_mem_bytes": RenderingServer.RENDERING_INFO_VIDEO_MEM_USED,
}
const MONITORS := {
	"objects": Performance.OBJECT_COUNT,
	"resources": Performance.OBJECT_RESOURCE_COUNT,
	"nodes": Performance.OBJECT_NODE_COUNT,
}

## The extension class, looked up at run time so older builds still load this script.
const COUNTERS := "RebuildCounters"

var _tree: SceneTree
var _frame := 0
var _usec := 0
var _godot := {}
var _atlas := []
var _atlas_changes := 0
## The GameClient whose `world_minutes` clock the light steps are counted on, if any.
var _client: Node
var _minutes := 0.0

func start(tree: SceneTree) -> void:
	_tree = tree
	_frame = Engine.get_process_frames()
	_usec = Time.get_ticks_usec()
	_godot = _godot_counters()
	_atlas = _shadow_atlas()
	_atlas_changes = 0
	_client = tree.root.find_child("WorldOfOsso", true, false)
	_minutes = _client.world_minutes() if _client else 0.0
	if not ClassDB.class_exists(COUNTERS):
		return
	if not tree.process_frame.is_connected(_sample_shadow_atlas):
		tree.process_frame.connect(_sample_shadow_atlas)
	ClassDB.class_call_static(COUNTERS, "take")

## `rebuilds_per_frame` (category means), `rebuild_sites` (first call site of each
## counted category), `world_events` (unit visuals that entered the scene: a window with
## any did not see a steady world) and `godot_per_frame` (allocation and pipeline-compilation deltas
## per frame; the memory and object values are net changes, Godot exposes no gross count
## except `objects_created`).
func finish() -> Dictionary:
	if not ClassDB.class_exists(COUNTERS):
		return {"rebuild_guard_error": COUNTERS + " is missing in this build"}
	var taken: Dictionary = ClassDB.class_call_static(COUNTERS, "take")
	_tree.process_frame.disconnect(_sample_shadow_atlas)
	var frames := maxi(Engine.get_process_frames() - _frame, 1)
	var counts: Dictionary = taken.counts
	counts["shadow_atlas"] = _atlas_changes
	var rebuilds := {}
	for category in counts:
		rebuilds[category] = float(counts[category]) / frames
	var godot := {"objects_created": float(taken.objects_created) / frames}
	var now := _godot_counters()
	for name in now:
		godot[name] = float(now[name] - _godot[name]) / frames
	var sites: Dictionary = taken.sites
	if _atlas_changes > 0:
		sites["shadow_atlas"] = "root viewport or directional shadow atlas settings changed"
	return {"rebuild_frames": frames, "rebuild_seconds": (Time.get_ticks_usec() - _usec) / 1e6,
		"rebuild_light_steps": _light_steps(), "rebuild_counts": counts, "rebuilds_per_frame": rebuilds, "rebuild_sites": sites,
		"world_events": taken.world_events,
		"godot_per_frame": godot}

## One line per category above its `ALLOWLIST` budget (0 when absent), with its call site.
static func violations(report: Dictionary) -> PackedStringArray:
	var failed := PackedStringArray()
	var frames: int = report.rebuild_frames
	# The takes fall mid-frame, so a per-frame operation may run once more than the frames
	# between them.
	var light_steps: int = report.rebuild_light_steps
	for category in report.rebuild_counts:
		var allowed: Dictionary = ALLOWLIST.get(category, {})
		var budget: int = allowed.get("per_frame", 0) * (frames + 1) + allowed.get("per_light_step", 0) * light_steps
		var count: int = report.rebuild_counts[category]
		if count > budget:
			failed.append("%s %d in %d frames (%.3f/frame) over budget %d at %s" % [category, count,
				frames, float(count) / frames, budget, report.rebuild_sites.get(category, "unknown site")])
	return failed

## The lit-time steps the world light may apply in the segment: the step boundaries the
## client clock crossed, plus one already due at the start and applied in the segment's
## first frame. Without a GameClient no world light runs; one step stays allowed.
func _light_steps() -> int:
	if _client == null:
		return 1
	var now: float = _client.world_minutes()
	if now < _minutes:
		now += DAY_HALF_MINUTES
	return floori(now / LIGHT_STEP_HALF_MINUTES) - floori(_minutes / LIGHT_STEP_HALF_MINUTES) + 1

func _godot_counters() -> Dictionary:
	var counters := {}
	for name in RENDERING_INFOS:
		counters[name] = RenderingServer.get_rendering_info(RENDERING_INFOS[name])
	for name in MONITORS:
		counters[name] = Performance.get_monitor(MONITORS[name])
	return counters

func _shadow_atlas() -> Array:
	var viewport := _tree.root
	var settings := [viewport.positional_shadow_atlas_size, viewport.positional_shadow_atlas_16_bits,
		ProjectSettings.get_setting(DIRECTIONAL_SIZE), ProjectSettings.get_setting(DIRECTIONAL_16_BITS)]
	for quadrant in 4:
		settings.append(viewport.get_positional_shadow_atlas_quadrant_subdiv(quadrant))
	return settings

func _sample_shadow_atlas() -> void:
	var now := _shadow_atlas()
	if now != _atlas:
		_atlas_changes += 1
		_atlas = now
