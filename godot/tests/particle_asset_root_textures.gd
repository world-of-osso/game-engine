extends "res://tests/m2_loader_pixels.gd"

# NPC, item and doodad particles read textures from their model's product asset root.
# The texture exists only under products/<product>/<build>/textures, with no flat
# textures/ copy; a model under another root must still report the texture missing.
const PRODUCT := FIXTURE + "/products/wow/fixturebuild"
const OTHER := FIXTURE + "/products/wow/otherbuild"

func run_cases() -> void:
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(PRODUCT + "/textures"))
	if not write_fixture(PRODUCT + "/textures/910001.blp", make_blp(Color.WHITE)):
		quit(1)
		return
	var passed := true
	if FileAccess.file_exists(FIXTURE + "/textures/910001.blp"):
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
	var missing = probe.asset_root_node(stage, OTHER + "/models/910010.m2")
	if not (missing is String and missing.contains("particle textures missing")):
		push_error("A model outside the texture's root bound it anyway: " + str(missing))
		passed = false
	stage.free()
	DirAccess.remove_absolute(ProjectSettings.globalize_path(PRODUCT + "/textures/910001.blp"))
	for folder in [PRODUCT + "/textures", PRODUCT, FIXTURE + "/products/wow", FIXTURE + "/products"]:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(folder))
	cleanup()
	print("PASS: particle textures resolve from the model asset root" if passed else "FAIL: particle textures resolve from the model asset root")
	quit(0 if passed else 1)
