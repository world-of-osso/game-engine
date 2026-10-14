extends SceneTree

var client: Node
var output: String
var started: int
var revealed := false

func _initialize() -> void:
	output = OS.get_environment("CHARSELECT_PROOF_DIR")
	assert(not output.is_empty(), "CHARSELECT_PROOF_DIR required")
	DirAccess.make_dir_recursive_absolute(output)
	root.size = Vector2i(1920, 1080)
	client = load(ProjectSettings.get_setting("application/run/main_scene")).instantiate()
	root.add_child(client)
	current_scene = client
	started = Time.get_ticks_msec()
	process_frame.connect(check_reveal)

func has_drawable(node: Node) -> bool:
	if node is MeshInstance3D and node.mesh != null and node.is_visible_in_tree():
		return true
	for child in node.get_children():
		if has_drawable(child):
			return true
	return false

func check_reveal() -> void:
	if Time.get_ticks_msec() - started > 180000:
		push_error("character scene reveal timed out")
		quit(1)
		return
	var ui = client.find_child("CharacterSelectUI", true, false)
	if ui == null or not ui.visible:
		return
	var scene = client.find_child("CharacterSelectScene", true, false)
	var model = scene.find_child("SelectedCharacter", true, false) if scene != null else null
	var terrain = scene.find_child("WorldTerrain", true, false) if scene != null else null
	if model == null or terrain == null or not has_drawable(model) or not has_drawable(terrain):
		push_error("CharacterSelectUI revealed before camp and character drawables")
		process_frame.disconnect(check_reveal)
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(output.path_join("empty-scene-red.png"))
		quit(1)
		return
	if not revealed:
		revealed = true
		process_frame.disconnect(check_reveal)
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(output.path_join("ready-scene.png"))
		print("PASS character scene revealed with camp and character drawables")
		quit(0)
