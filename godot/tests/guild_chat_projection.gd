extends "res://tests/communities_fidelity.gd"

## Offline projection/input smoke only. Authority is the existing32-member fixture;
## this does not establish live delivery, history or officer privacy.
func capture() -> bool:
	root.size = Vector2i(1920, 1080)
	var directory := OS.get_environment("GUILD_CHAT_PROJECTION_SHOTS")
	if not completion.require(not directory.is_empty(), "Owned projection output required"):
		return false
	DirAccess.make_dir_recursive_absolute(directory)
	for skin in ["modern", "forever"]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		if not completion.require(ui.call("show_guild_communities_preview", skin == "forever") == ""):
			return false
		if not completion.step(await settle(), "initial projection"):
			return false
		if not completion.step(await click(ui, "CommunitiesChatTab"), "physical Chat tab"):
			return false
		for entry in [
			["CommunitiesChatMessageFrame", Rect2(754, 394, 416, 303)],
			["CommunitiesChatEditBox", Rect2(750, 701, 423, 32)],
			["CommunitiesStreamDropdown", Rect2(748, 358, 160, 25)],
			["CommunitiesChatMemberList", Rect2(1202, 390, 139, 335)],
		]:
			if not completion.step(assert_rect(ui, entry[0], entry[1]), "actual native source rectangle"):
				return false
		if not completion.step(await click(ui, "CommunitiesStreamDropdown"), "physical stream menu"):
			return false
		if not completion.require(control(ui, "CommunitiesOfficerChatChoice").is_visible_in_tree(), "Officer choice must actually paint"):
			return false
		if not completion.step(await click(ui, "CommunitiesOfficerChatChoice"), "physical Officer stream"):
			return false
		if not completion.require(ui.call("frame_text", "CommunitiesStreamDropdownText") == "Officer Chat"):
			return false
		if not completion.step(await click(ui, "CommunitiesChatEditBox"), "physical composer"):
			return false
		if not completion.require(control(ui, "CommunitiesChatEditBox").has_focus(), "Native composer focus"):
			return false
		if not completion.step(capture_png(directory, skin + "-offline-chat-projection.png"), "offline projection capture"):
			return false
		ui.free()
		if not completion.step(await settle(), "skin teardown"):
			return false
		if not completion.case_done():
			return false
	return true
