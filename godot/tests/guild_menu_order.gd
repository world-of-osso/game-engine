extends "res://tests/guild_control_fidelity.gd"

# Actual projected CanvasItem order, not a declaration/enum assertion.
func capture() -> bool:
	root.size = Vector2i(1920, 1080)
	for forever in [false, true]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_forever_guild_ranks_preview" if forever else "show_guild_ranks_preview")
		if not completion.require(error == "", error):
			return false
		if not completion.step(await settle(), "mount settings control"):
			return false
		if not completion.step(await click(ui, "GuildControlUINavigationDropdown"), "open other caller"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await settle(), "project navigation menu"):
			return false
		if not menu_draws_above(ui, "GuildControlNavigationMenuBackground", ["GuildControlUIRankOrderFrameRank1Label"]):
			return false
		ui.free()
		ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		error = ui.call("show_guild_communities_preview", forever)
		if not completion.require(error == "", error):
			return false
		if not completion.step(await settle(), "mount roster"):
			return false
		if not completion.step(await click(ui, "GuildMember1"), "open real lower-ranked fixture member"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await settle(), "project detail"):
			return false
		if not completion.step(await click(ui, "GuildMemberDetailRankDropdown"), "open direct rank menu"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == ""):
			return false
		if not completion.step(await settle(), "project direct rank menu"):
			return false
		if not menu_draws_above(ui, "GuildMemberDetailRankMenuBackground", ["GuildMemberDetailOnline", "GuildMemberDetailNoteText", "GuildMemberDetailOfficerNoteText"]):
			return false
		if not completion.case_done():
			return false
		ui.free()
	print("PASS GUILD_MENU_ORDER both_skins=true guild_rank_above_notes=true navigation_control=true")
	return true

# Headless order checks need projected controls, not a rendered-frame signal.
func settle() -> bool:
	for frame in range(4):
		await process_frame
	return true

func menu_draws_above(ui: Node, background: String, covered_names: Array) -> bool:
	var menu := control(ui, background)
	for name in covered_names:
		var covered := control(ui, name)
		if not completion.require(menu.is_visible_in_tree() and covered.is_visible_in_tree(), "visible overlapping menu/control " + str(name)):
			return false
		if not completion.require(menu.z_index > covered.z_index, background + " draws below " + str(name) + ": menu=" + str(menu.z_index) + " content=" + str(covered.z_index)):
			return false
	return true
