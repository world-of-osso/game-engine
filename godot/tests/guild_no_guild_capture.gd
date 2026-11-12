extends SceneTree

## Capture the production Communities renderer with an offline NotInGuild reply.
func _initialize() -> void:
	call_deferred("capture_forever")

func capture_forever() -> void:
	root.size = Vector2i(1920, 1080)
	var directory := OS.get_environment("GUILD_NO_GUILD_CAPTURE")
	if directory.is_empty():
		push_error("GUILD_NO_GUILD_CAPTURE is required")
		quit(1)
		return
	if not DirAccess.dir_exists_absolute(directory):
		if DirAccess.make_dir_recursive_absolute(directory) != OK:
			push_error("Cannot create capture directory: " + directory)
			quit(1)
			return
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	if ui == null:
		push_error("RegistryUi is unavailable")
		quit(1)
		return
	root.add_child(ui)
	var error: String = ui.call("show_forever_guild_no_guild_preview")
	if not error.is_empty():
		push_error(error)
		quit(1)
		return
	for frame in range(12):
		await process_frame
		await RenderingServer.frame_post_draw
	if ui.find_child("GuildFinderFrame", true, false) == null:
		push_error("Guild Finder frame is not rendered")
		quit(1)
		return
	for name in ["GuildRosterTab", "CommunitiesGuildEntry", "CommunitiesMemberListScrollBox"]:
		if ui.find_child(name, true, false) != null:
			push_error("Guild-only frame remains visible: " + name)
			quit(1)
			return
	for entry in [
		["CommunitiesGuildFinderEntryName", "Guild Finder"],
		["CommunitiesFindCommunityEntryName", "Find a Community"],
		["CommunitiesJoinCommunityEntryName", "Join or create a Community"],
	]:
		if str(ui.call("frame_text", entry[0])) != entry[1]:
			push_error("Unexpected Communities entry: " + entry[0])
			quit(1)
			return
	await RenderingServer.frame_post_draw
	var screenshot := root.get_texture().get_image()
	var path := directory.path_join("forever-after.png")
	if screenshot.get_size() != Vector2i(1920, 1080) or screenshot.save_png(path) != OK:
		push_error("Could not save Forever capture: " + path)
		quit(1)
		return
	print("PASS FOREVER_NO_GUILD_CAPTURE ", path)
	ui.queue_free()
	quit(0)
