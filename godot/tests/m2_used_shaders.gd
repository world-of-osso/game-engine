extends SceneTree

# Godot compiles an M2 shader variant when a material first takes it, 25-50 ms of
# main-thread time, so the first model of each pipeline stalled character select and
# world entry. Each pipeline a model uses is recorded in user://, and a later run
# compiles the recorded pipelines ahead of need, one per call.

const DATA := "res://../data/models/"
# Single-batch doodad model (FDID 1016191).
const MODEL := 1016191
const PIPELINES := "user://m2_shader_pipelines.txt"
# Mod blend, two-sided, no depth test or write: a pipeline the model does not use.
const RECORDED := "4 1 0 0"

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
		FileAccess.open(PIPELINES, FileAccess.WRITE).store_string(saved)
	elif FileAccess.file_exists(PIPELINES):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(PIPELINES))

func lines() -> PackedStringArray:
	return FileAccess.get_file_as_string(PIPELINES).split("\n", false)

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
	if FileAccess.file_exists(PIPELINES):
		saved = FileAccess.get_file_as_string(PIPELINES)
	FileAccess.open(PIPELINES, FileAccess.WRITE).store_string(RECORDED + "\n")
	var loader = ClassDB.instantiate("WowAssetLoader")
	var compiled := compile_all(loader)
	if compiled != 1:
		fail("compiled %d pipelines from a file recording 1" % compiled)
		return
	var result: Dictionary = loader.load_m2(DATA + "%d.m2" % MODEL)
	if result.has("error"):
		fail("%d load: %s" % [MODEL, result.error])
		return
	var recorded := lines()
	if recorded.size() < 2 or recorded[0] != RECORDED:
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
	if compiled != recorded.size() - 1:
		fail("compiled %d of the model's %d pipelines" % [compiled, recorded.size() - 1])
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
	print("PASS: model %d recorded %s after %s; each recorded pipeline compiled once" % [
		MODEL, recorded.slice(1), RECORDED])
	restore()
	quit(0)
