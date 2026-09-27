extends RefCounted

const SCREENSHOT := "res://../data/diagnostics/godot-conversion/inworld-selected-player.png"
const MIN_MODEL_SAMPLES := 200
const MIN_WEAPON_SAMPLES := 5

func find_visual(player: Node3D) -> Node3D:
	for child in player.get_children():
		if child is Node3D and child.find_child("Skeleton3D", true, false) is Skeleton3D:
			return child
	return null

func inspect_visual(visual: Node3D, equipped: bool) -> String:
	var skeleton := visual.find_child("Skeleton3D", true, false) as Skeleton3D
	if skeleton == null or skeleton.get_bone_count() == 0:
		return "Replicated player lacks authored character skeleton"
	var body_meshes := 0
	for node in visual.find_children("*", "MeshInstance3D", true, false):
		var mesh := node as MeshInstance3D
		var parent := mesh.get_parent()
		var under_equipment := false
		while parent != visual:
			if parent.name.begins_with("Equipment"):
				under_equipment = true
				break
			parent = parent.get_parent()
		if not under_equipment and mesh.is_visible_in_tree() and mesh.mesh != null and mesh.mesh.get_surface_count() > 0:
			body_meshes += 1
	if body_meshes == 0:
		return "Replicated player lacks visible authored body mesh"
	var equipment := visual.find_children("Equipment*", "Node3D", true, false)
	if not equipped:
		if not equipment.is_empty():
			return "Empty replicated equipment retained equipment nodes: " + str(equipment.size())
		return ""
	for hand_name in ["EquipmentMainHand", "EquipmentOffHand"]:
		var hand := visual.find_child(hand_name, true, false) as Node3D
		if hand == null or not has_visible_mesh(hand):
			return "Replicated starter hand lacks visible authored model: " + hand_name
	return ""

func has_visible_mesh(parent: Node3D) -> bool:
	for node in parent.find_children("*", "MeshInstance3D", true, false):
		var mesh := node as MeshInstance3D
		if mesh.is_visible_in_tree() and mesh.mesh != null and mesh.mesh.get_surface_count() > 0:
			return true
	return false

func capture_initial(tree: SceneTree, player: Node3D, visual: Node3D) -> String:
	if DisplayServer.get_name() == "headless":
		return "Replicated player GPU proof requires a real display"
	var viewport := SubViewport.new()
	viewport.size = Vector2i(512, 512)
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.world_3d = player.get_world_3d()
	tree.root.add_child(viewport)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 3.5
	viewport.add_child(camera)
	camera.look_at_from_position(player.global_position + Vector3(0, 1.4, 5), player.global_position + Vector3.UP * 1.2)
	camera.make_current()
	var was_paused := tree.paused
	tree.paused = true
	var shown := await capture_frame(viewport)
	var save_error := shown.save_png(SCREENSHOT)
	if save_error != OK:
		tree.paused = was_paused
		viewport.free()
		return "Could not save replicated player screenshot: " + error_string(save_error)
	var hands := [visual.find_child("EquipmentMainHand", true, false), visual.find_child("EquipmentOffHand", true, false)]
	for hand in hands:
		hand.visible = false
	var without_hands := await capture_frame(viewport)
	for hand in hands:
		hand.visible = true
	visual.visible = false
	var without_model := await capture_frame(viewport)
	visual.visible = true
	tree.paused = was_paused
	viewport.free()
	var model_pixels := count_changed_pixels(shown, without_model)
	var weapon_pixels := count_changed_pixels(shown, without_hands)
	if model_pixels < MIN_MODEL_SAMPLES or weapon_pixels < MIN_WEAPON_SAMPLES:
		return "Replicated player/hand GPU pixels missing: model=" + str(model_pixels) + " hands=" + str(weapon_pixels)
	print("PASS: selected world player GPU model samples=", model_pixels, " hand samples=", weapon_pixels, " screenshot=", SCREENSHOT)
	return ""

func capture_frame(viewport: SubViewport) -> Image:
	for frame in 2:
		await RenderingServer.frame_post_draw
	return viewport.get_texture().get_image()

func count_changed_pixels(first: Image, second: Image) -> int:
	var changed := 0
	for y in range(0, first.get_height(), 2):
		for x in range(0, first.get_width(), 2):
			if not first.get_pixel(x, y).is_equal_approx(second.get_pixel(x, y)):
				changed += 1
	return changed
