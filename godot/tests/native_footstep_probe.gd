extends RefCounted

const SAMPLE_IDS := ["540120", "540121", "540127", "540202"]

func check(tree: SceneTree, client: Node, player: Node3D, locomotion: RefCounted) -> String:
	var sound := client.get_node_or_null("NativeSound")
	var footsteps := sound.get_node_or_null("Footsteps") if sound != null else null
	if footsteps == null:
		return "NativeSound has no owned spatial Footsteps channel"
	for frame in range(12):
		await tree.process_frame
	if footsteps.get_child_count() != 0:
		return "Idle Stand emitted a footstep"
	var error: String = await check_run(tree, player, locomotion, footsteps, 0.8, true)
	if error != "":
		return error
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null or not camera.current:
		return "No current world camera for spatial audio listener"
	print("PASS: real GameClient Run phase/terrain/catalog/Ogg/3D position and gain; idle/stop quiet; camera current")
	return ""

func check_run(tree: SceneTree, player: Node3D, locomotion: RefCounted, footsteps: Node, gain: float, should_play: bool) -> String:
	if footsteps.get_child_count() != 0:
		return "Previous step remained active before the next run"
	var input := InputEventKey.new()
	input.physical_keycode = KEY_W
	input.pressed = true
	tree.root.push_input(input, true)
	var observed_ids := {}
	var error := ""
	for frame in range(100):
		await tree.process_frame
		for child in footsteps.get_children():
			if child is AudioStreamPlayer3D and child.is_playing():
				if not should_play:
					error = "Muted Run emitted a footstep"
				elif locomotion.animation.current_animation_id() != 5:
					error = "Footstep played outside selected Run 5"
				elif child.global_position.distance_to(player.global_position) > 0.1:
					error = "Footstep source missed actual player world position"
				elif not child.stream is AudioStreamOggVorbis or not SAMPLE_IDS.has(child.stream.resource_name):
					error = "Footstep did not decode an extracted local Ogg: " + str(child.stream)
				elif absf(child.volume_linear - gain) > 0.025:
					error = "Run footstep ignored master/effects gain: " + str(child.volume_linear)
				observed_ids[child.get_instance_id()] = true
		if error != "":
			break
	input.pressed = false
	tree.root.push_input(input, true)
	if error != "":
		return error
	for frame in range(80):
		await tree.process_frame
	if locomotion.animation.current_animation_id() != 0 or footsteps.get_child_count() != 0:
		return "Stopped movement retained footsteps or did not return to Stand"
	if should_play and observed_ids.size() < 2:
		return "Repeated Run halves did not yield multiple independent emitters: " + str(observed_ids.size())
	return ""
