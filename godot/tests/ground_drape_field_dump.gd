extends SceneTree

# Field dump of draped spell-kit geometry, for byte-level comparison between two client
# builds (master vs. a drape-query change): the Battle Shout cast base (kit 279043, model
# cfx_warrior_battleshout_castbase, five flat batches) spawned through the production kit
# path at fixed points on terrain, a rug doodad, WMO stair treads, a wall foot and the
# Stormwind training ground. Offline: no server, real authored map.
#
# Per point and batch it prints `DRAPE_FIELD <point> <batch> vertices=<n> indices=<n> sha256=<hex>`
# over the exact bytes of the vertex, normal, UV, UV2 and index arrays, and writes the
# arrays themselves to DRAPE_DUMP_DIR/<point>-<batch>.bin when that variable is set.
# Two builds drape identically when their DRAPE_FIELD lines are identical.
#
# Each point is then cast again where it stood. The ground did not change, so the recast
# must come from the drape cache (`drape_cache_hits` grows) and its batches must hash
# exactly like the fresh drape; it prints `DRAPE_RECAST <point> hits=<n>`.

const SHOUT_VISUAL := 190617
const SHOUT_KIT := 279043
const SHOUT_SPELL := 6673
const POINTS := {
	"flat": Vector3(-8590.0, 96.9, -770.0),
	"rug": Vector3(-8550.4, 108.76, -720.9),
	"stairs": Vector3(-8564.0, 107.4, -778.0),
	"wall": Vector3(-8559.0, 99.6, -770.0),
	"training": Vector3(-8746.2, 100.9, -333.34),
}

var client: Node3D
var closed := false

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.size = Vector2i(640, 360)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_ms(1500)
	client.preview_set_drape(true)
	var dump_dir := OS.get_environment("DRAPE_DUMP_DIR")
	if dump_dir != "":
		DirAccess.make_dir_recursive_absolute(dump_dir)
	for label in POINTS:
		var point: Vector3 = POINTS[label]
		if not await load_world(point):
			return
		var meshes := await spawn_draped(point)
		if closed:
			return
		if meshes.is_empty():
			fail("%s: the kit draped no batch" % label)
			return
		var fresh: Array[String] = []
		for mesh in meshes:
			fresh.append(dump_batch(label, mesh, dump_dir))
		client.preview_clear_kits()
		await wait_ms(300)
		if not await recast_matches(label, point, fresh):
			return
	finish()

## Casts the kit again at `point` and checks it reused the cached drape byte for byte.
func recast_matches(label: String, point: Vector3, fresh: Array[String]) -> bool:
	var hits_before: int = client.spell_visuals_state().drape_cache_hits
	var meshes := await spawn_draped(point)
	if closed:
		return false
	var hits: int = client.spell_visuals_state().drape_cache_hits - hits_before
	var recast: Array[String] = []
	for mesh in meshes:
		recast.append(batch_hash(mesh))
	client.preview_clear_kits()
	await wait_ms(300)
	print("DRAPE_RECAST %s hits=%d" % [label, hits])
	if hits == 0:
		fail("%s: the recast draped again instead of reusing the cache" % label)
		return false
	if recast != fresh:
		fail("%s: cached drape %s differs from fresh drape %s" % [label, recast, fresh])
		return false
	return true

func load_world(point: Vector3) -> bool:
	client.preview_set_focus(point)
	var error: String = client.preview_world_map("Azeroth", point)
	if error != "":
		fail("map Azeroth: " + error)
		return false
	var deadline := Time.get_ticks_msec() + 240000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.terrain_height_at(point.x, point.z) != null \
				and client.account_state().terrain.pending_count == 0 and objects_idle():
			# Let the render surfaces of the last spawned objects settle.
			await wait_ms(1500)
			return true
	fail("world never finished loading at %s" % point)
	return false

func objects_idle() -> bool:
	var debug: Dictionary = JSON.parse_string(client.preview_objects_debug())
	return debug.waiting_assets == 0 and debug.pending == 0 and debug.loading == 0 \
		and debug.waiting == 0 and debug.spawned_wmos + debug.spawned_doodads > 0

## Spawns the kit, freezes its animation, and returns its batches once their draped
## geometry stopped changing for 30 frames.
func spawn_draped(point: Vector3) -> Array[MeshInstance3D]:
	var deadline := Time.get_ticks_msec() + 120000
	var error := "preparing"
	while error != "" and Time.get_ticks_msec() < deadline:
		await process_frame
		error = client.preview_spell_kit(SHOUT_VISUAL, SHOUT_KIT, SHOUT_SPELL, point)
	if error != "":
		fail("spell kit: " + error)
		return []
	var stable := 0
	var last := ""
	while Time.get_ticks_msec() < deadline:
		await process_frame
		pause_kits()
		var meshes := kit_meshes()
		var shape := ",".join(meshes.map(func(mesh: MeshInstance3D): return str(vertex_count(mesh))))
		stable = stable + 1 if shape == last and not meshes.is_empty() else 0
		last = shape
		if stable >= 30:
			return meshes
	fail("the kit's batches never settled")
	return []

func kit_meshes() -> Array[MeshInstance3D]:
	var meshes: Array[MeshInstance3D] = []
	for child in client.get_children():
		if str(child.name).begins_with("PreviewKit%d_%d" % [SHOUT_VISUAL, SHOUT_KIT]):
			for node in child.find_children("*", "MeshInstance3D", true, false):
				meshes.append(node as MeshInstance3D)
	meshes.sort_custom(func(a: MeshInstance3D, b: MeshInstance3D): return str(a.name) < str(b.name))
	return meshes

func pause_kits() -> void:
	for child in client.get_children():
		if str(child.name).begins_with("PreviewKit"):
			for player in child.find_children("*", "WowAnimationPlayer", true, false):
				player.call("set_paused", true)

func vertex_count(mesh: MeshInstance3D) -> int:
	var array_mesh := mesh.mesh as ArrayMesh
	if array_mesh == null or array_mesh.get_surface_count() == 0:
		return 0
	return array_mesh.surface_get_array_len(0)

## Prints the batch's DRAPE_FIELD line and returns its hash.
func dump_batch(label: String, mesh: MeshInstance3D, dump_dir: String) -> String:
	var bytes := batch_bytes(mesh)
	var sha := batch_hash(mesh)
	var indices = (mesh.mesh as ArrayMesh).surface_get_arrays(0)[Mesh.ARRAY_INDEX]
	print("DRAPE_FIELD %s %s vertices=%d indices=%d sha256=%s" % [label, mesh.name, vertex_count(mesh),
		indices.size() if indices != null else 0, sha])
	if dump_dir != "":
		var file := FileAccess.open(dump_dir.path_join("%s-%s.bin" % [label, mesh.name]), FileAccess.WRITE)
		file.store_buffer(bytes)
		file.close()
	return sha

func batch_bytes(mesh: MeshInstance3D) -> PackedByteArray:
	var arrays := (mesh.mesh as ArrayMesh).surface_get_arrays(0)
	var bytes := PackedByteArray()
	for kind in [Mesh.ARRAY_VERTEX, Mesh.ARRAY_NORMAL, Mesh.ARRAY_TEX_UV, Mesh.ARRAY_TEX_UV2, Mesh.ARRAY_INDEX]:
		var value = arrays[kind]
		if value != null:
			bytes.append_array(var_to_bytes(value))
	return bytes

func batch_hash(mesh: MeshInstance3D) -> String:
	var hashing := HashingContext.new()
	hashing.start(HashingContext.HASH_SHA256)
	hashing.update(batch_bytes(mesh))
	return hashing.finish().hex_encode()

func wait_ms(ms: int) -> void:
	var deadline := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < deadline:
		await process_frame

func finish() -> void:
	if closed:
		return
	closed = true
	client.free()
	print("PASS: ground drape field dump")
	quit(0)

func fail(error: String) -> void:
	if closed:
		return
	closed = true
	push_error(error)
	if is_instance_valid(client):
		client.free()
	quit(1)
