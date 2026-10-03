extends RefCounted

# Menu Escape clears selection before opening. This probe covers the exposed
# authored setting -> next physical self-selection boundary, not live edits of
# an unchanged existing ring. No movement or new fixture protocol markers.
var fixture

func run(flow: SceneTree, client: Node, player: Node3D) -> bool:
	fixture = flow
	if client.target_state().target != null or client.get_node_or_null("GameMenuUI") != null:
		return reject("Target-marker probe requires target-none/menu-closed")
	await tap(KEY_F1)
	var selected: Dictionary = client.target_state()
	if selected.target == null or selected.target_name != fixture.NAME or selected.circle_on != selected.target:
		return reject("Physical default TargetSelf did not select fixture player: " + str(selected))
	var target_id = selected.target
	var baseline: Dictionary = frame_snapshot(client)
	if not frame_present(baseline):
		return reject("Self-selection lacks visible authored TargetFrame: " + str(baseline))
	if not await expect_ring(client, player, target_id, baseline, true, "default On"):
		return false
	if not await clear_selection(client, player):
		return false
	if not await set_marker(client, false):
		return false
	await tap(KEY_F1)
	if not await expect_ring(client, player, target_id, baseline, false, "authored Show Target Marker Off"):
		return false
	if not await clear_selection(client, player):
		return false
	if not await set_marker(client, true):
		return false
	await tap(KEY_F1)
	if not await expect_ring(client, player, target_id, baseline, true, "authored Show Target Marker On"):
		return false
	if not await clear_selection(client, player):
		return false
	if client.get_node_or_null("GameMenuUI") != null:
		return reject("Target-marker probe left menu open")
	print("PASS: authored Show Target Marker Off/On preserves self-selection and TargetFrame, hides/restores newly selected Decal")
	return true

func expect_ring(client: Node, player: Node3D, target_id, baseline: Dictionary, shown: bool, phase: String) -> bool:
	var ring := player.get_node_or_null("TargetCircle") as Decal
	if ring == null:
		return reject("%s: selected player lacks valid TargetCircle Decal" % phase)
	var ring_id := ring.get_instance_id()
	# Observe several native sync passes: Off must hide, not free/recreate.
	for frame in range(5):
		await fixture.process_frame
		var state: Dictionary = client.target_state()
		if state.target != target_id or state.target_name != fixture.NAME or state.circle_on != target_id:
			return reject("%s: marker option lost self-selection/ring ownership: %s" % [phase, state])
		if frame_snapshot(client) != baseline:
			return reject("%s: marker option changed TargetFrame: %s" % [phase, frame_snapshot(client)])
		if not is_instance_valid(ring) or player.get_node_or_null("TargetCircle") != ring or ring.get_instance_id() != ring_id or ring.is_queued_for_deletion():
			return reject("%s: selected Decal freed/replaced rather than retained" % phase)
		if ring.visible != shown or ring.is_visible_in_tree() != shown:
			return reject("%s: native TargetCircle remains visible=%s (tree=%s), expected %s; self-selection and TargetFrame unchanged" % [phase, ring.visible, ring.is_visible_in_tree(), shown])
	return true

func frame_snapshot(client: Node) -> Dictionary:
	var ui := client.get_node_or_null("UnitFramesUI")
	if ui == null:
		return {}
	var frame := ui.find_child("TargetFrame", true, false) as Control
	var name := ui.find_child("TargetName", true, false) as Label
	var health := ui.find_child("TargetHealthBarText", true, false) as Label
	if frame == null or name == null or health == null:
		return {}
	return {"visible": frame.is_visible_in_tree(), "rect": frame.get_global_rect(), "name": name.text, "name_visible": name.is_visible_in_tree(), "health": health.text, "health_visible": health.is_visible_in_tree()}

func frame_present(state: Dictionary) -> bool:
	# Health text visibility follows the Status Text setting (Retail default None: hover only).
	return not state.is_empty() and state.visible and state.name_visible and state.name == fixture.NAME

func clear_selection(client: Node, player: Node3D) -> bool:
	await tap(KEY_ESCAPE)
	for frame in range(3):
		await fixture.process_frame
	if client.target_state().target != null or client.target_state().circle_on != null or player.get_node_or_null("TargetCircle") != null or client.get_node_or_null("GameMenuUI") != null:
		return reject("Escape did not clear self-selection/ring without opening menu")
	return true

func set_marker(client: Node, enabled: bool) -> bool:
	await tap(KEY_ESCAPE)
	if not await fixture.wait_menu(client) or not fixture.menu_authored(client):
		return false
	for name in ["MenuBtnOptions", "OptionsTabhud", "ToggleSwitchshow_target_markerRightHit" if enabled else "ToggleSwitchshow_target_markerLeftHit"]:
		if not await click_option(client, name):
			return false
	if client.target_state().target != null:
		return reject("Authored marker edit unexpectedly selected a target")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		return reject("Target-marker probe requires fixture-owned XDG_CONFIG_HOME")
	var path := config.path_join("world-of-osso/options_settings.ron")
	var saved := FileAccess.get_file_as_string(path).replace(" ", "").replace("\t", "").replace("\n", "").replace("\r", "")
	var expected := "show_target_marker:true" if enabled else "show_target_marker:false"
	if not saved.contains(expected):
		return reject("Authored marker edit did not persist %s before closing Options" % expected)
	if not await click_option(client, "OptionsDoneButton"):
		return false
	if client.get_node_or_null("GameMenuUI") != null:
		if not await click_option(client, "MenuBtnResume"):
			return false
	return await fixture.wait_menu_closed(client, null)

func click_option(client: Node, name: String) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	var control := menu.find_child(name, true, false) as Control if menu != null else null
	if control == null or not control.is_visible_in_tree():
		return reject("Authored marker Options control absent/hidden: " + name)
	await fixture.click(control)
	return true

func tap(key: Key) -> void:
	fixture.push_key(key, true)
	await fixture.process_frame
	fixture.push_key(key, false)
	for frame in range(3):
		await fixture.process_frame

func reject(message: String) -> bool:
	fixture.fail(message)
	return false
