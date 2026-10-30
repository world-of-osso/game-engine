extends "res://tests/guild_control_fidelity.gd"

# GuildTabItems0 is a read-only label while its tab lacks View and an editable box with it.
# Both projections carry the frame name, so the swap must leave one control named GuildTabItems0.
func capture() -> bool:
	root.size = Vector2i(1920, 1080)
	for forever in [true]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_forever_guild_ranks_preview" if forever else "show_guild_ranks_preview")
		if not completion.require(error == "", error):
			return false
		if not completion.step(await settle(), "mount Guild Control"):
			return false
		for name in ["GuildControlUINavigationDropdown", "GuildControlPage1", "GuildControlUIRankSettingsFrameRankDropdown", "GuildControlRankChoice2", "GuildControlUINavigationDropdown", "GuildControlPage2"]:
			if not completion.step(await click(ui, name), "click " + name):
				return false
			if not completion.require(ui.call("guild_control_preview_dispatch") == "", name):
				return false
		if not completion.step(await settle(), "project bank tab permissions"):
			return false
		if not completion.require(control(ui, "GuildTabItems0") is LineEdit, "Member tab 1 starts viewable"):
			return false
		for view in [false, true]:
			if not completion.step(await click(ui, "GuildTabView0"), "click View"):
				return false
			var request: String = ui.call("guild_control_preview_dispatch")
			if not completion.require(request.contains("view: %s" % view), request):
				return false
			if not completion.require(ui.call("guild_control_preview_reply") == ""):
				return false
			if not completion.step(await settle(), "project View=%s" % view):
				return false
			var fields: Array = ui.find_children("GuildTabItems0", "Control", true, false)
			if not completion.require(fields.size() == 1, "one GuildTabItems0 control, found %s" % [fields]):
				return false
			if not completion.require((fields[0] is LineEdit) == view, "GuildTabItems0 with View=%s is %s" % [view, fields[0]]):
				return false
		if not completion.case_done():
			return false
		ui.free()
	print("PASS GUILD_CONTROL_FIELD_SWAP both_skins=true")
	return true

# Node identity and names need projected controls, not a rendered-frame signal.
func settle() -> bool:
	for frame in range(4):
		await process_frame
	return true
