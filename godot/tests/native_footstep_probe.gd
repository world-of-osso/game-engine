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
	var input := InputEventKey.new()
	input.physical_keycode = KEY_W
	input.pressed = true
	tree.root.push_input(input, true)
	var observed := false
	for frame in range(100):
		await tree.process_frame
		for child in footsteps.get_children():
			if child is AudioStreamPlayer3D and child.is_playing():
				if locomotion.animation.current_animation_id() != 5:
					return "Footstep played outside selected Run 5"
				if child.global_position.distance_to(player.global_position) > 0.1:
					return "Footstep source missed actual player world position"
				if not child.stream is AudioStreamOggVorbis or not SAMPLE_IDS.has(child.stream.resource_name):
					return "Footstep did not decode an extracted local Ogg: " + str(child.stream)
				if absf(child.volume_linear - 0.8) > 0.001:
					return "Run footstep did not use master times effects gain"
				observed = true
	if not observed:
		return "Real movement did not produce spatial catalog playback"
	input.pressed = false
	tree.root.push_input(input, true)
	for frame in range(80):
		await tree.process_frame
	if locomotion.animation.current_animation_id() != 0 or footsteps.get_child_count() != 0:
		return "Stopped movement retained footsteps or did not return to Stand"
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null or not camera.current:
		return "No current world camera for spatial audio listener"
	print("PASS: real GameClient Run phase/terrain/catalog/Ogg/3D position and gain; idle/stop quiet; camera current")
	return ""
