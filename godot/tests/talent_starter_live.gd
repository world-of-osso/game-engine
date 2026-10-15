extends "res://tests/talent_import_export_live.gd"

# Fresh private Mage, level10; actual UDP activation, online level changes,
# guided deviation and relog. Uses only the explicitly supplied private admin.
func _run() -> void:
	directory = OS.get_environment("TALENT_LOADOUT_CAPTURE_PATH")
	skin = OS.get_environment("TALENT_LOADOUT_SKIN")
	var endpoint := OS.get_environment("TALENT_LOADOUT_ENDPOINT")
	if directory.is_empty() or not endpoint.ends_with(":5881"):
		abort("Starter proof requires private UDP5881 and capture directory")
		return
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await frames(120)
	await login(endpoint)
	await expect_caption("Default Loadout")
	await menu()
	await control("TalentStarterBuild")
	if root.find_child("TalentLoadoutEdit4294967295", true, false) != null:
		abort("Starter must not have a user-config gear")
		return
	await capture("starter-menu")
	await click("TalentStarterBuild")
	await expect_caption("Starter Build")
	await node_rank(62122, "1/1")
	await node_rank(102467, "0/1")
	await capture("starter-level10")
	set_level(11)
	await node_rank(102467, "1/1")
	await expect_caption("Starter Build")
	await capture("starter-level11-auto-spend")
	var at_eleven := rank_state()
	await login(endpoint)
	await expect_caption("Starter Build")
	if rank_state() != at_eleven:
		abort("Starter level11 relog changed complete rank/entry projection")
		return
	set_level(80)
	await node_rank(62123, "2/2")
	await expect_caption("Starter Build")
	await capture("starter-level80")
	await click("HeroSpecButton")
	var choices := root.find_children("TalentChoiceNode*Entry*Spell*Button", "Button", true, false)
	var choice_name := ""
	for candidate in choices:
		if candidate.is_visible_in_tree() and not candidate.disabled:
			choice_name = str(candidate.name)
			break
	if choice_name.is_empty():
		abort("Installed Starter leaves no selectable hero deviation")
		return
	await click(choice_name)
	await control("TalentStarterDeviationText")
	await capture("starter-deviation-confirmation")
	await click("TalentLoadoutCancel")
	await expect_caption("Starter Build")
	await click(choice_name)
	await click("TalentLoadoutSave")
	await expect_caption("Default Loadout")
	var apply_buttons := root.find_children("TalentHero*Apply", "Button", true, false)
	var applied := false
	for button in apply_buttons:
		if button.is_visible_in_tree() and not button.disabled:
			await click(str(button.name))
			applied = true
			break
	if not applied:
		abort("Hero deviation did not expose authoritative Apply")
		return
	await frames(120)
	await expect_caption("Default Loadout")
	# Authoritative allocation snapshots close the hero overlay; Escape here
	# would instead close the whole Talents window.
	await control("TalentLoadoutDropDown")
	var edited := rank_state()
	await login(endpoint)
	await expect_caption("Default Loadout")
	if rank_state() != edited:
		abort("Accepted Starter deviation did not persist full allocation after relog")
		return
	await capture("starter-deactivated-relog")
	evidence.append({"starter_activation":true,"online_level10_to11_to80":true,"starter_relog_equal":true,"deviation_cancel_then_continue":true,"deactivated_relog_equal":true})
	var file := FileAccess.open(directory.path_join(skin + "-starter-proof.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(evidence, "\t"))
	file.close()
	print("TALENT_STARTER_LIVE_PASS ", skin)
	quit(0)

func node_rank(node_id: int, expected: String) -> void:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var label := root.find_child("TalentNode" + str(node_id) + "Ranks", true, false) as Label
		if label != null and label.is_visible_in_tree() and label.text == expected:
			evidence.append({"node":node_id,"rank":expected})
			return
		await process_frame
	abort("Node " + str(node_id) + " did not reach " + expected)

func set_level(level: int) -> void:
	var admin := OS.get_environment("TALENT_STARTER_ADMIN")
	var character := OS.get_environment("TALENT_STARTER_CHARACTER")
	if admin.is_empty() or character.is_empty() or OS.get_environment("GAME_SERVER_ADMIN_SOCKET") != "/tmp/game-server-admin-5881.sock":
		abort("Private Starter admin path, socket and character required")
		return
	var output: Array = []
	var result := OS.execute(admin, ["set-level", character, str(level)], output, true)
	if result != 0:
		abort("Private level change failed: " + str(output))
		return
	evidence.append({"admin_level":level,"response":output})
