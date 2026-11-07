extends SceneTree
# A Stormwind guard (creature display 3167, HD human male 1011653) loads 107 batches;
# batches with equal merge keys bind one material per build (rust/src/assets/mod.rs
# load_batch): the build makes 16 ShaderMaterials, not one or more per batch (116).
const DISPLAY := 3167
const MAX_MATERIALS := 20

func _initialize() -> void:
	call_deferred("run")

func fail(reason: String) -> void:
	push_error("M2_SHARED_MATERIALS_FAIL " + reason)
	quit(1)

func run() -> void:
	var counters = ClassDB.instantiate("RebuildCounters")
	counters.take()
	var loader = ClassDB.instantiate("WowAssetLoader")
	var loaded: Dictionary = loader.load_creature_display(DISPLAY)
	if loaded.has("error"):
		fail(str(loaded.error))
		return
	var created: int = counters.take().counts.shader_material
	var model: Node = loaded.node
	var batches := model.find_children("*", "MeshInstance3D", true, false)
	var by_material := {}
	for batch: MeshInstance3D in batches:
		var material := batch.get_active_material(0) as ShaderMaterial
		if material == null:
			continue
		by_material.get_or_add(material.get_instance_id(), []).append(batch)
	print("M2_SHARED_MATERIALS batches=%d materials=%d created=%d" % [batches.size(), by_material.size(), created])
	if created == 0 or created > MAX_MATERIALS:
		fail("display %d built %d ShaderMaterials for %d batches, expected 1..%d" % [DISPLAY, created, batches.size(), MAX_MATERIALS])
		return
	model.free()
	print("M2_SHARED_MATERIALS_PASS")
	quit(0)
