extends "res://tests/target_marker_options_probe.gd"

# Authored capture -> persisted action ownership -> physical self-selection.
# SetTarget acknowledgement is outside this fixture's consumer coverage.
func run(flow: SceneTree, client: Node, player: Node3D) -> bool:
	fixture = flow
	if not expect_unselected(client, player, "initial"):
		return false
	await tap(KEY_F1)
	var selected: Dictionary = client.target_state()
	if selected.target == null or selected.target_name != fixture.NAME or selected.circle_on != selected.target:
		return reject("Default F1 did not select fixture player: " + str(selected))
	var target_id = selected.target
	var baseline: Dictionary = frame_snapshot(client)
	if not frame_present(baseline):
		return reject("Default F1 lacks visible authored TargetFrame: " + str(baseline))
	if not await expect_ring(client, player, target_id, baseline, true, "baseline F1"):
		return false
	if not await clear_selection(client, player):
		return false
	if not await capture_binding(client, KEY_T, "F1", "T", "key:KeyT"):
		return false
	await tap(KEY_F1)
	if not expect_unselected(client, player, "old F1 after T capture"):
		return false
	await tap(KEY_T)
	if not await expect_ring(client, player, target_id, baseline, true, "captured T"):
		return false
	if not await clear_selection(client, player):
		return false
	if not await capture_binding(client, KEY_F1, "T", "F1", "key:F1"):
		return false
	await tap(KEY_T)
	if not expect_unselected(client, player, "old T after F1 restoration"):
		return false
	await tap(KEY_F1)
	if not await expect_ring(client, player, target_id, baseline, true, "restored F1"):
		return false
	if not await clear_selection(client, player):
		return false
	if not expect_unselected(client, player, "final"):
		return false
	print("PASS: authored TargetSelf F1 -> T -> F1, exact labels/canonical ownership, inactive old keys, same fixture-player ring and TargetFrame; selection cleared/menu closed/input released (no server acknowledgement proof)")
	return true

func capture_binding(client: Node, key: Key, before: String, after: String, token: String) -> bool:
	# Selection was cleared separately; this Escape opens, not clears, the menu.
	await tap(KEY_ESCAPE)
	if not await fixture.wait_menu(client) or not fixture.menu_authored(client):
		return false
	for name in ["MenuBtnOptions", "OptionsTabkeybindings"]:
		if not await click_binding_control(client, name):
			return false
	var section := binding_control(client, "KeybindingSectiontargeting")
	var button := binding_control(client, "KeybindingSectiontargetingButton")
	if section == null or button == null or not section.is_visible_in_tree() or not section.is_ancestor_of(button):
		return reject("Authored Targeting section frame/button missing or not nested")
	if not await click_binding_control(client, "KeybindingSectiontargetingButton"):
		return false
	if not expect_label(client, before):
		return false
	if not await click_binding_control(client, "KeybindingButtontarget_self"):
		return false
	if not expect_label(client, "Press a key\u2026"):
		return false
	await tap(key)
	if not expect_label(client, after) or not expect_saved_binding(token):
		return false
	if not expect_no_selection(client, "during authored capture"):
		return false
	if not await click_binding_control(client, "OptionsDoneButton"):
		return false
	if client.get_node_or_null("GameMenuUI") != null:
		if not await click_binding_control(client, "MenuBtnResume"):
			return false
	return await fixture.wait_menu_closed(client, null)

func expect_saved_binding(token: String) -> bool:
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		return reject("TargetSelf probe requires fixture-owned XDG_CONFIG_HOME")
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		return reject("TargetSelf canonical options file missing: " + path)
	# Parse serialized external action/value pairs, not unrelated token substrings.
	var pairs := RegEx.new()
	var error := pairs.compile('([A-Za-z0-9_]+)\\s*:\\s*(?:Some\\("([^"\\r\\n]+)"\\)|None)')
	if error != OK:
		return reject("Compile canonical binding-pair expression: " + error_string(error))
	var target_values: Array[String] = []
	var owners: Array[String] = []
	for pair in pairs.search_all(FileAccess.get_file_as_string(path)):
		var action := pair.get_string(1)
		var value := pair.get_string(2)
		if action == "TargetSelf":
			target_values.append(value)
		if value == token:
			owners.append(action)
	if target_values != [token] or owners != ["TargetSelf"]:
		return reject("Canonical TargetSelf must uniquely own %s; values=%s owners=%s" % [token, target_values, owners])
	print("TARGET_BINDING_PROBE canonical TargetSelf=Some(\"%s\") owners=%s" % [token, owners])
	return true

func expect_label(client: Node, expected: String) -> bool:
	var label := binding_control(client, "KeybindingButtonTexttarget_self") as Label
	if label == null or not label.is_visible_in_tree() or label.text != expected:
		return reject("Authored TargetSelf label expected '%s', got '%s'" % [expected, label.text if label != null else "<missing>"])
	return true

func expect_no_selection(client: Node, phase: String) -> bool:
	var state: Dictionary = client.target_state()
	var frame := frame_snapshot(client)
	if state.target != null or state.circle_on != null or (not frame.is_empty() and frame.visible):
		return reject("%s: unexpected selection/ring/TargetFrame: %s %s" % [phase, state, frame])
	return true

func expect_unselected(client: Node, player: Node3D, phase: String) -> bool:
	if not expect_no_selection(client, phase):
		return false
	if player.get_node_or_null("TargetCircle") != null or client.get_node_or_null("GameMenuUI") != null:
		return reject(phase + ": probe left player ring or menu open")
	return true

func binding_control(client: Node, name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu.find_child(name, true, false) as Control if menu != null else null

func click_binding_control(client: Node, name: String) -> bool:
	var control := binding_control(client, name)
	if control == null or not control.is_visible_in_tree():
		return reject("Authored TargetSelf control absent/hidden: " + name)
	await fixture.click(control)
	return true
