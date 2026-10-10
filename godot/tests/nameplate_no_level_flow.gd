extends SceneTree

# Native tree acceptance through Nameplates::sync_nodes, without a server.
# NAMEPLATE_LEVEL_SHOTS contains only requested PNGs; phase labels RED/GREEN captures.
var host: Node3D
var failed := false

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(720, 600)
	var tree_only := OS.get_environment("NAMEPLATE_LEVEL_TREE_ONLY") == "1"
	for forever in [true, false]:
		host = Node3D.new()
		root.add_child(host)
		var error: String = NameplateProbe.draw_level_cases(host, OS.get_environment("NAMEPLATE_LEVEL_DATA"), forever)
		if not error.is_empty():
			fail(error)
			return
		await process_frame
		await process_frame
		if not tree_only:
			await RenderingServer.frame_post_draw
			var skin := "forever" if forever else "modern"
			var shot := root.get_texture().get_image()
			var filename := "%s-%s.png" % [skin, OS.get_environment("NAMEPLATE_LEVEL_PHASE")]
			if shot.save_png(OS.get_environment("NAMEPLATE_LEVEL_SHOTS").path_join(filename)) != OK:
				fail("Cannot save " + filename)
				return
		for plate: Control in host.get_node("Nameplates").get_children():
			check_plate(plate, forever)
		host.free()
	if failed:
		print("FAIL: nameplate level acceptance")
		quit(1)
	else:
		if tree_only:
			print("TREE ONLY: rendered capture omitted")
		print("PASS: ordinary full-width Forever without numeric level/badge; skull geometry, selection, classification and Modern retained")
		quit(0)

func check_plate(plate: Control, forever: bool) -> void:
	var fill := plate.get_child(0) as TextureRect
	var frame := plate.get_child(1) as NinePatchRect
	var name := plate.get_child(2) as Label
	var health := plate.get_child(3) as Label
	var classification := plate.get_child(5) as TextureRect
	var skull_case := name.text in ["World Boss", "Large Gap"]
	var selected := name.text in ["Ordinary", "World Boss"]
	var body_width := 160.0 if forever and skull_case else 188.0
	expect(absf(fill.size.x - body_width * 0.96) < 0.001, "%s %s body width %s expected %s" % [forever, name.text, fill.size.x / 0.96, body_width])
	expect(name.is_visible_in_tree() and not name.text.is_empty(), "Name lost")
	expect(health.is_visible_in_tree() and health.text == "96%", "Health percent lost")
	expect(frame.is_visible_in_tree(), "Health border lost")
	expect(classification.is_visible_in_tree() == (name.text in ["Ordinary", "Rare", "World Boss"]), "Classification lost for " + name.text)
	if classification.is_visible_in_tree():
		expect(is_equal_approx(classification.get_global_rect().end.x, fill.get_global_rect().position.x), "Classification reserves hidden raid-marker space for " + name.text)
	var level := plate.get_node_or_null("PlayerLevelDiffFrame") as Control
	if not forever:
		expect(level == null, "Modern gained level frame")
		expect(frame.size.is_equal_approx(Vector2(198, 24)), "Modern frame changed")
		expect(fill.position.is_equal_approx(Vector2(-94, -9.5)), "Modern fill changed")
		return
	expect(frame.size.is_equal_approx(Vector2(body_width + 2, 22)), "Forever 1px frame changed")
	if level == null:
		fail("Forever skull/selected frame removed")
		return
	var icon := level.get_node("playerLevelDiffIcon") as TextureRect
	var ring := level.get_node("selectedBorder") as TextureRect
	var skull := level.get_node("highLevelTexture") as TextureRect
	var text := level.get_node_or_null("playerLevelDiffText") as Label
	expect(text == null or not text.is_visible_in_tree(), "Numeric level visible for " + name.text)
	expect(icon.is_visible_in_tree() == skull_case, "Ordinary level badge remains for " + name.text)
	expect(skull.is_visible_in_tree() == skull_case, "Skull visibility changed for " + name.text)
	expect(ring.is_visible_in_tree() == (selected and skull_case), "Badge selected ring visible without skull for " + name.text)
	if not skull_case:
		expect(not level.is_visible_in_tree(), "Ordinary level-indicator root visible for " + name.text)
		for part: Control in level.get_children():
			expect(not part.is_visible_in_tree(), "Ordinary indicator art visible for " + name.text)
			expect(not part.is_visible_in_tree() or not part.get_global_rect().intersects(health.get_global_rect()), "Indicator overlaps health percent for " + name.text)
	if selected and skull_case:
		expect(ring.get_rect() == Rect2(65, -13.5, 30, 27), "Selected border geometry changed")
	if skull_case:
		expect(skull.get_rect() == Rect2(68.5, -11.5, 23, 23), "Skull geometry changed")

func expect(condition: bool, message: String) -> void:
	if not condition:
		fail(message)

func fail(message: String) -> void:
	failed = true
	push_error("Nameplate level: " + message)
