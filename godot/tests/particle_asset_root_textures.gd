extends "res://tests/m2_loader_pixels.gd"

# NPC, item and doodad particles read textures from their model's product asset root.
# The texture exists only under products/<product>/<build>/textures, owned by a
# receipt, with no flat textures/ copy; a flat-root model must report it missing.
const ROOT := "user://particle_asset_root_fixture"
const KEY := "0123456789abcdef0123456789abcdef"
const PRODUCT := ROOT + "/products/wow/" + KEY
const TEXTURE := "products/wow/" + KEY + "/textures/910001.blp"

func write_receipt(bytes: int) -> bool:
	var receipt := {"product": "wow", "build": "12.1.5.70688", "build_key": KEY,
		"fdid": 910001, "kind": "blp", "path": TEXTURE, "bytes": bytes,
		"sha256": "a".repeat(64), "content_key": "b".repeat(32)}
	var index := JSON.stringify({"version": 1, "assets": [receipt]})
	return write_fixture(ROOT + "/cache/model-asset-index.json", index.to_utf8_buffer())

func run_cases() -> void:
	for folder in ["/cache", "/textures", "/products/wow/" + KEY + "/textures"]:
		DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(ROOT + folder))
	var blp := make_blp(Color.WHITE)
	if not write_fixture(ROOT + "/" + TEXTURE, blp) or not write_receipt(blp.size()):
		quit(1)
		return
	var passed := true
	if FileAccess.file_exists(ROOT + "/textures/910001.blp"):
		push_error("A flat textures/910001.blp copy exists; the fixture must not have one")
		passed = false
	var stage := Node3D.new()
	root.add_child(stage)
	var probe: Object = ClassDB.instantiate("WowParticleProbe")
	var placed = probe.asset_root_node(stage, PRODUCT + "/models/910010.m2")
	if not placed is Node3D:
		push_error("Product-root particle texture not bound: " + str(placed))
		passed = false
	elif placed.find_child("Particles910010_0", true, false) == null:
		push_error("Product-root placement created no particle pool")
		passed = false
	var missing = probe.asset_root_node(stage, ROOT + "/models/910010.m2")
	if not (missing is String and missing.contains("particle textures missing")):
		push_error("A flat-root model bound a texture it has no copy of: " + str(missing))
		passed = false
	stage.free()
	for path in [TEXTURE, "cache/model-asset-index.json", "products/wow/" + KEY + "/textures",
			"products/wow/" + KEY, "products/wow", "products", "textures", "cache", ""]:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(ROOT + "/" + path))
	print("PASS: particle textures resolve from the model asset root" if passed else "FAIL: particle textures resolve from the model asset root")
	quit(0 if passed else 1)
