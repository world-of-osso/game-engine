extends "res://tests/guild_control_fidelity.gd"

# Retail: clicking a button or list row never takes keyboard focus, so Enter afterwards
# reaches the game and opens chat (GameClient::open_chat_from_key in unhandled input).
# Keyboard focus still reaches rows from the keyboard, and Enter there activates the row.

class EnterProbe extends Node:
	var enters := 0
	func _unhandled_key_input(event: InputEvent) -> void:
		var key := event as InputEventKey
		if key.pressed and not key.echo and key.keycode == KEY_ENTER:
			enters += 1

func capture() -> bool:
	root.size = Vector2i(1920, 1080)
	var probe := EnterProbe.new()
	root.add_child(probe)
	for forever in [true]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_guild_communities_preview", forever)
		if not completion.require(error == "", error):
			return false
		if not completion.step(await settle(), "mount Communities"):
			return false
		var enters := probe.enters
		if not completion.step(await click(ui, "CommunitiesChatTab"), "pointer click on a row"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == "", "chat tab click"):
			return false
		if not completion.require(root.gui_get_focus_owner() == null, "pointer click left focus on %s" % root.gui_get_focus_owner()):
			return false
		if not completion.step(await key(KEY_ENTER), "Enter after the click"):
			return false
		if not completion.require(probe.enters == enters + 1, "Enter after a row click must reach the game (chat)"):
			return false
		var info := control(ui, "GuildInfoTab")
		info.grab_focus()
		if not completion.step(await key(KEY_ENTER), "Enter on a keyboard-focused row"):
			return false
		if not completion.require(ui.call("guild_control_preview_dispatch") == "", "info tab activation"):
			return false
		if not completion.step(await settle(), "project Info tab"):
			return false
		if not completion.require(probe.enters == enters + 1, "Enter on a keyboard-focused row must not reach chat"):
			return false
		if not completion.require(ui.find_child("CommunitiesGuildLogButton", true, false) != null, "Enter activates the keyboard-focused Info tab"):
			return false
		if not completion.case_done():
			return false
		ui.free()
		await process_frame
	print("PASS SELECTION_ROW_POINTER_FOCUS both_skins=true")
	return true

func settle() -> bool:
	for frame in range(4):
		await process_frame
	return true
