extends SceneTree

# Freeing an M2 whose batch materials have no other owner must not make the
# RenderingServer read a freed material. Godot 4.7 MeshInstance3D releases its
# surface override materials before freeing its RenderingServer instance, and
# RendererSceneCull::free first updates every dirty instance, so a batch built
# in the same frame (or never drawn, as under --headless) dereferenced its freed
# material: four `Parameter "material" is null` errors per batch
# (godotengine/godot#85817). World streaming, equipment swaps and player
# removal free models this way.

const DATA := "res://../data/models/"
# Single-batch doodad model with a skeleton (FDID 1016191).
const MODEL := 1016191


class ErrorObserver:
	extends Logger
	var mutex := Mutex.new()
	var errors: Array[String] = []

	func _log_error(
		function: String,
		_file: String,
		_line: int,
		_code: String,
		rationale: String,
		_editor_notify: bool,
		error_type: int,
		_script_backtraces: Array[ScriptBacktrace]
	) -> void:
		if error_type != ERROR_TYPE_WARNING:
			mutex.lock()
			errors.append("%s: %s" % [function, rationale])
			mutex.unlock()

	func take() -> Array[String]:
		mutex.lock()
		var result := errors.duplicate()
		errors.clear()
		mutex.unlock()
		return result


var observer := ErrorObserver.new()


func fail(message: String) -> void:
	push_error(message)
	quit(1)


func _initialize() -> void:
	OS.add_logger(observer)
	if not ClassDB.class_exists("WowAssetLoader"):
		fail("WowAssetLoader not registered")
		return
	check.call_deferred()


func _finalize() -> void:
	OS.remove_logger(observer)


func load_model(loader: Object) -> Node3D:
	var result: Dictionary = loader.load_m2(DATA + "%d.m2" % MODEL)
	if result.has("error"):
		fail("%d load: %s" % [MODEL, result.error])
		return null
	return result.node


# Frees `model` and returns whether every batch material was released with it,
# so the case exercises the last reference going away during the free.
func free_releasing_materials(model: Node3D) -> bool:
	var materials: Array[WeakRef] = []
	for batch in model.find_children("Batch*", "MeshInstance3D", false, false):
		materials.append(weakref((batch as MeshInstance3D).get_active_material(0)))
	model.free()
	return not materials.is_empty() and materials.all(func(material: WeakRef) -> bool: return material.get_ref() == null)


func run_case(name: String, model: Node3D) -> bool:
	if model == null:
		return false
	if not free_releasing_materials(model):
		fail("%s: batch materials outlived the freed model" % name)
		return false
	var errors := observer.take()
	if not errors.is_empty():
		fail("%s: freeing the model logged %d errors: %s" % [name, errors.size(), errors])
		return false
	return true


func check() -> void:
	var loader = ClassDB.instantiate("WowAssetLoader")
	if not run_case("never added", load_model(loader)):
		return
	var placed := load_model(loader)
	if placed == null:
		return
	root.add_child(placed)
	if not run_case("added and freed in one frame", placed):
		return
	print("PASS: M2 models freed in the frame they were built release their batch materials without RenderingServer errors")
	quit(0)
