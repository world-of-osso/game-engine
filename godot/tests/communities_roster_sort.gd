extends "res://tests/communities_fidelity.gd"
# Offline snapshots through production controls/reducer; not a live guild authority proof.

func capture() -> void:
	var directory := OS.get_environment("COMMUNITIES_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["forever"]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.call("show_guild_communities_preview", skin == "forever") == "")
		await settle()
		assert_rect(ui, "CommunitiesRosterColumn0", Rect2(748, 385, 40, 24))
		assert_rect(ui, "CommunitiesRosterColumn1", Rect2(786, 385, 45, 24))
		assert_rect(ui, "CommunitiesMemberListScrollBox", Rect2(749, 410, 592, 315))
		await click(ui, "CommunitiesRosterColumn0")
		assert(ui.call("frame_text", "GuildMember0Level") == "20")
		capture_png(directory, skin + "-level-ascending.png")
		await click(ui, "CommunitiesRosterColumn0")
		assert(ui.call("frame_text", "GuildMember0Level") == "80")
		capture_png(directory, skin + "-level-descending.png")
		await click(ui, "CommunitiesRosterColumn1")
		assert_member_class_group(ui, 3) # Warrior1, not alphabetic Druid11.
		capture_png(directory, skin + "-class-ascending.png")
		await click(ui, "CommunitiesRosterColumn1")
		assert_member_class_group(ui, 2) # Druid11.
		capture_png(directory, skin + "-class-descending.png")
		var selected := String(ui.call("frame_text", "GuildMember0Name"))
		await click(ui, "GuildMember0", MOUSE_BUTTON_RIGHT)
		assert(String(ui.call("guild_communities_preview_receipt")).contains(selected))
		assert(ui.find_child("GuildMemberPromote", true, false) != null)
		ui.queue_free()
		await settle()
	print("PASS: both-skin physical Level/Class ascending/descending, numeric class IDs, unchanged source geometry and sorted member action identity")
	quit(0)

func assert_member_class_group(ui: Node, expected: int) -> void:
	var name := String(ui.call("frame_text", "GuildMember0Name"))
	assert(name.begins_with("Member"))
	# Preview classes are Paladin2/Mage8/Druid11/Warrior1; no tie-order claim.
	assert(name.substr(6).to_int() % 4 == expected, name)
