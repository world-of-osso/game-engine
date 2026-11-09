extends SceneTree

## Ruby placement 36449083 uses this real collision-only M2. Its empty authored
## render box is min=+1e7/max=-1e7; it must not become a huge surface grid.
const FDID := 4695280
const MAX_QUERY_BYTES := 1024 * 1024

func _initialize() -> void:
	call_deferred("run_fixture")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func run_fixture() -> void:
	var probe: Node3D = ClassDB.instantiate("WowRibbonWorldProbe")
	root.add_child(probe)
	var error: String = probe.load_model(
		"res://../data/models/%d.m2" % FDID,
		"res://../data/models/%d00.skin" % FDID, FDID)
	if error != "":
		fail(error)
		return
	var bodies := probe.find_children("*", "StaticBody3D", true, false)
	var meshes := probe.find_children("*", "MeshInstance3D", true, false)
	if bodies.size() != 1 or not meshes.is_empty():
		fail("Expected real camera collision without render geometry")
		return
	var before := Performance.get_monitor(Performance.MEMORY_STATIC)
	var surface: Dictionary = probe.surface_census(3.0)
	var query_bytes := Performance.get_monitor(Performance.MEMORY_STATIC) - before
	if surface.get("height") != 3.0 or surface.get("doodad_cells") != 0:
		fail("Collision-only doodad entered render surface index: " + str(surface))
		return
	if query_bytes > MAX_QUERY_BYTES:
		fail("Empty render surface query retained %d bytes" % query_bytes)
		return
	if not is_instance_valid(bodies[0]) or bodies[0].get_child_count() != 1:
		fail("Surface query removed the camera collision shape")
		return
	print("PASS: Ruby collision-only M2 preserves collision, returns terrain floor, and retains 0 surface cells; query_bytes=", query_bytes)
	probe.free()
	quit(0)
