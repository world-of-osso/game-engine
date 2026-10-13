extends "res://tests/housing_expert_source_live.gd"

# Owned private fixture cleanup through normal native Options, never config-file mutation.
func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server!="127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Expert binding cleanup requires owned private fixture configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if not await configure_expert_bindings(true): return
	print("PASS: private5870 own prior Expert fixture bindings cleared through native Options ",housing_skin)
	client.free()
	quit(0)
