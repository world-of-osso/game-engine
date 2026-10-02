extends SceneTree

# Godot compiles a shader when a material first takes it, 10-50 ms of main-thread time,
# so the first model of each M2 pipeline, WMO variant, the terrain and each liquid
# stalled character select and world entry. Each shader the scene uses is recorded in user://, and a later
# run compiles the recorded shaders ahead of need, one per call.

const DATA := "res://../data/models/"
# Single-batch doodad model (FDID 1016191).
const MODEL := 1016191
const USED := "user://used_shaders.txt"
# An M2 pipeline the model does not use (Mod blend, two-sided, no depth test or write),
# a WMO variant (two-sided, blended, clamped T) and a liquid shader.
const RECORDED := ["m2 4 1 0 0", "wmo 1 1 0 1", "resource res://shaders/water.gdshader"]

var saved = null

func fail(message: String) -> void:
	push_error(message)
	restore()
	quit(1)

func _initialize() -> void:
	if not ClassDB.class_exists("WowAssetLoader"):
		fail("WowAssetLoader not registered")
		return
	check.call_deferred()

func restore() -> void:
	if saved != null:
		FileAccess.open(USED, FileAccess.WRITE).store_string(saved)
	elif FileAccess.file_exists(USED):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(USED))

func lines() -> PackedStringArray:
	return FileAccess.get_file_as_string(USED).split("\n", false)

## Calls until nothing is left to compile; the number compiled, or -1 on error.
func compile_all(loader: Object) -> int:
	var compiled := 0
	while true:
		var result: Dictionary = loader.compile_next_used_shader()
		if result.has("error"):
			fail("compile: %s" % result.error)
			return -1
		if not result.compiled:
			return compiled
		compiled += 1
	return compiled

func check() -> void:
	if FileAccess.file_exists(USED):
		saved = FileAccess.get_file_as_string(USED)
	FileAccess.open(USED, FileAccess.WRITE).store_string("\n".join(RECORDED) + "\n")
	var loader = ClassDB.instantiate("WowAssetLoader")
	var compiled := compile_all(loader)
	if compiled != RECORDED.size():
		fail("compiled %d shaders from a file recording %d" % [compiled, RECORDED.size()])
		return
	var result: Dictionary = loader.load_m2(DATA + "%d.m2" % MODEL)
	if result.has("error"):
		fail("%d load: %s" % [MODEL, result.error])
		return
	var recorded := lines()
	if recorded.size() <= RECORDED.size() or recorded.slice(0, RECORDED.size()) != PackedStringArray(RECORDED):
		fail("model %d recorded no pipeline after %s: %s" % [MODEL, RECORDED, recorded])
		return
	var unique := {}
	for line in recorded:
		unique[line] = true
	if unique.size() != recorded.size():
		fail("pipelines recorded twice: %s" % [recorded])
		return
	# The model's own shaders compiled as it loaded; its scenery-fade shaders did not.
	compiled = compile_all(loader)
	var model_pipelines := recorded.size() - RECORDED.size()
	if compiled != model_pipelines:
		fail("compiled %d of the model's %d pipelines" % [compiled, model_pipelines])
		return
	result.node.free()
	result = loader.load_m2(DATA + "%d.m2" % MODEL)
	if result.has("error"):
		fail("%d reload: %s" % [MODEL, result.error])
		return
	result.node.free()
	if lines() != recorded:
		fail("reloading the model changed the record: %s" % [lines()])
		return
	print("PASS: model %d recorded %s after %s; each recorded shader compiled once" % [
		MODEL, recorded.slice(RECORDED.size()), RECORDED])
	restore()
	quit(0)
