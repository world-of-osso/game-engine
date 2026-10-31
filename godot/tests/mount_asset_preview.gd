extends SceneTree
# Authentic private server catalog + native Mount Journal selection; no injected models.
var client: Node
var failures: Array = []
var results: Array = []
var out := OS.get_environment("MOUNT_ASSET_OUTPUT")

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_fixture")

func run_fixture() -> void:
	root.size = Vector2i(1280, 720)
	if out.is_empty() or OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5910":
		finish("Explicit private server/evidence required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, 180000):
		finish("asset startup timeout")
		return
	var error: String = client.connect_account("127.0.0.1:5910", "fb_mount_model_assets", "fbtest", false)
	if not error.is_empty():
		finish(error)
		return
	if not await wait_until(func(): return client.get_node_or_null("CharacterSelectUI") != null, 120000):
		finish("character select timeout")
		return
	await click("CharCard_1")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and int(client.mount_journal_state().catalog_count) > 0 and control("MinimapLauncherButton") != null, 180000):
		finish("world/catalog timeout")
		return
	var ready := FileAccess.open(OS.get_environment("MOUNT_ASSET_READY"), FileAccess.WRITE)
	ready.store_string(str(client.account_state()))
	ready.close()
	if not await wait_until(func(): return FileAccess.file_exists(OS.get_environment("MOUNT_ASSET_GO")), 180000):
		finish("owned diagnostic setup timeout")
		return
	if client.account_state().ui_skin != "Forever":
		finish("wrong active skin: " + str(client.account_state().ui_skin))
		return
	await click("MinimapLauncherButton")
	await click("LauncherEntryCollectionsMicroButton")
	if not await wait_until(func(): return control("MountJournalSearchBox") != null, 10000):
		finish("journal missing")
		return
	# Fixture-only routing: Mount Journal's host does not forward scroll input yet.
	# UiAuditProbe forwards authentic inputs to this SAME RegistryUi/controller.
	# Remove this reparenting when the normal journal host wires scroll_list_input.
	var audit := ClassDB.instantiate("UiAuditProbe") as Node
	root.add_child(audit)
	var journal = client.get_node("CollectionsJournalUI")
	journal.reparent(audit)
	journal.name = "QuestOverflowUI"
	var targets = JSON.parse_string(FileAccess.get_file_as_string(OS.get_environment("MOUNT_ASSET_TARGETS")))
	for target in targets:
		if target.get("hide_collected", false):
			await toggle_collected_filter()
		var search = control("MountJournalSearchBox")
		search.text = target.name
		search.emit_signal("text_changed", target.name)
		var row := "MountJournalRow%d" % int(target.mount_id)
		await frames(8)
		for fraction in [0.0, 0.25, 0.5, 0.75, 1.0]:
			if row_visible(row):
				break
			await drag_list_thumb(fraction)
		if not row_visible(row):
			root.get_texture().get_image().save_png(out + "/missing-row-%d.png" % int(target.mount_id))
			var names: Array = []
			for node in root.find_children("*MountJournal*", "", true, false):
				names.append({"name": str(node.name), "type": node.get_class(), "rect": str(node.get_global_rect()) if node is Control else ""})
			failures.append({"mount_id": target.mount_id, "reason": "authentic catalog row not visible after exact-name search", "search_text": str(control("MountJournalSearchBox").text), "nodes": names})
			continue
		await click(row)
		if not await wait_until(func(): return int(client.mount_journal_state().selected) == int(target.mount_id) and client.mount_journal_state().preview_ready, 90000):
			failures.append({"mount_id": target.mount_id, "reason": "detached preview did not become ready", "selected": client.mount_journal_state().selected})
			continue
		if target.get("rotate", false):
			await rotate_preview()
		await frames(int(target.get("settle_frames", 6)))
		var model_host = control("MountJournalModelScene")
		var viewport = find_viewport(model_host)
		if viewport == null:
			failures.append({"mount_id": target.mount_id, "reason": "no preview SubViewport"})
			continue
		await RenderingServer.frame_post_draw
		var image: Image = viewport.get_texture().get_image()
		var stats := pixel_stats(image)
		var meshes := mesh_count(viewport)
		var path := out + "/mount-%d.png" % int(target.mount_id)
		var save_error := image.save_png(path)
		if meshes == 0 or int(stats.opaque_samples) < 20 or save_error != OK:
			failures.append({"mount_id": target.mount_id, "reason": "empty geometry/pixels or failed capture", "meshes": meshes, "pixels": stats, "save_error": save_error})
		results.append({"mount_id": target.mount_id, "name": target.name, "requested_model_fdid": target.model_fdid, "skin": expected_skin, "meshes": meshes, "pixels": stats, "yaw": client.mount_journal_state().yaw, "image": path})
		if target.get("sample", false):
			var full: Image = root.get_texture().get_image()
			if full.get_width() > 1280:
				full.resize(1280, roundi(float(full.get_height()) * 1280.0 / full.get_width()))
			full.save_png(out + "/journal-%d.png" % int(target.mount_id))
		print("MOUNT_ASSET_RENDER ", target.mount_id, " meshes=", meshes, " pixels=", stats)
		if target.get("hide_collected", false):
			await toggle_collected_filter()
	finish("")

func control(name: String) -> Control:
	return root.find_child(name, true, false) as Control

func click(name: String) -> void:
	if not await wait_until(func(): return control(name) != null, 10000):
		return
	var at := control(name).get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = at
	root.push_input(motion)
	await frames(2)
	for pressed in [true, false]:
		var button := InputEventMouseButton.new()
		button.position = at
		button.button_index = MOUSE_BUTTON_LEFT
		button.pressed = pressed
		button.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		root.push_input(button)
		await frames(2)

func toggle_collected_filter() -> void:
	await click("MountJournalFilterButton")
	await click("MountJournalFilter0")
	await click("MountJournalFilterButton")
	await frames(8)

func rotate_preview() -> void:
	var host = control("MountJournalModelScene")
	var at := host.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = at
	root.push_input(motion)
	await frames(2)
	var button := InputEventMouseButton.new()
	button.position = at
	button.button_index = MOUSE_BUTTON_LEFT
	button.button_mask = MOUSE_BUTTON_MASK_LEFT
	button.pressed = true
	root.push_input(button)
	await frames(2)
	motion = InputEventMouseMotion.new()
	motion.position = at + Vector2(157, 0)
	motion.relative = Vector2(157, 0)
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion)
	await frames(2)
	button = InputEventMouseButton.new()
	button.position = motion.position
	button.button_index = MOUSE_BUTTON_LEFT
	button.pressed = false
	root.push_input(button)
	await frames(4)

func row_visible(name: String) -> bool:
	var row = control(name)
	var list = control("MountJournalScrollBox")
	return row != null and list != null and list.get_global_rect().has_point(row.get_global_rect().get_center())

func drag_list_thumb(fraction: float) -> void:
	var thumb = control("MountJournalScrollBoxScrollThumb")
	var track = control("MountJournalScrollBoxScrollTrack")
	if thumb == null or track == null:
		return
	var at := thumb.get_global_rect().position + Vector2(1.0, thumb.size.y / 2.0)
	var target := track.get_global_rect().position + Vector2(1.0, thumb.size.y / 2.0 + fraction * (track.size.y - thumb.size.y))
	var motion := InputEventMouseMotion.new()
	motion.position = at
	root.push_input(motion)
	await frames(2)
	var button := InputEventMouseButton.new()
	button.position = at
	button.button_index = MOUSE_BUTTON_LEFT
	button.button_mask = MOUSE_BUTTON_MASK_LEFT
	button.pressed = true
	root.push_input(button)
	await frames(2)
	motion = InputEventMouseMotion.new()
	motion.position = target
	motion.relative = target - at
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion)
	await frames(2)
	button = InputEventMouseButton.new()
	button.position = target
	button.button_index = MOUSE_BUTTON_LEFT
	button.pressed = false
	root.push_input(button)
	await frames(8)

func find_viewport(node: Node) -> SubViewport:
	if node is SubViewport:
		return node
	for child in node.get_children():
		var found := find_viewport(child)
		if found != null:
			return found
	return null

func mesh_count(node: Node) -> int:
	var count := 1 if node is MeshInstance3D and node.mesh != null and node.visible else 0
	for child in node.get_children():
		count += mesh_count(child)
	return count

func pixel_stats(image: Image) -> Dictionary:
	var opaque := 0
	var colors := {}
	for y in range(0, image.get_height(), 4):
		for x in range(0, image.get_width(), 4):
			var color := image.get_pixel(x, y)
			if color.a > 0.1:
				opaque += 1
				colors[color.to_html(false)] = true
	return {"opaque_samples": opaque, "unique_colors": colors.size(), "width": image.get_width(), "height": image.get_height()}

func frames(count: int) -> void:
	for _i in count:
		await process_frame

func wait_until(predicate: Callable, timeout: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		if predicate.call():
			return true
		await frames(2)
	return false

func finish(error: String) -> void:
	if not error.is_empty():
		failures.append({"reason": error})
	var file := FileAccess.open(out + "/results.json", FileAccess.WRITE)
	file.store_string(JSON.stringify({"renders": results, "failures": failures}, "\t"))
	file.close()
	print("MOUNT_ASSET_BATCH_DONE renders=", results.size(), " failures=", failures.size())
	quit(0 if failures.is_empty() else 1)
