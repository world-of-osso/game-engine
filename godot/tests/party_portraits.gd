extends SceneTree

# Offline process proof: real masked heads, roster reorders and viewport lifetimes.
var fixture: Node
var output := OS.get_environment("GODOT_CAPTURE_PATH")
var skin := OS.get_environment("GODOT_CAPTURE_SCREEN")

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	fixture = ClassDB.instantiate("PartyPortraitFixture")
	root.add_child(fixture)
	check_error(fixture.initialize(skin.begins_with("forever")))
	check_error(fixture.set_members(["Theron", "Jaina", "Valeera", "Uther"], true, true))
	await settle()
	check(count_views() == 0, "compact style has zero portrait hosts")
	print("PASS portrait_party_compact_zero_hosts")
	check_error(fixture.set_members(["Theron"], false, true))
	await wait_for_heads(1)
	check_binding("Theron", 0)
	check_error(fixture.set_members(["Theron", "Jaina", "Valeera", "Uther"], false, true))
	await wait_for_heads(4)
	for index in range(4):
		check_binding(["Theron", "Jaina", "Valeera", "Uther"][index], index)
	check_error(fixture.set_members(["Uther", "Jaina", "Theron"], false, true))
	await wait_for_heads(3)
	check_binding("Uther", 0)
	check_binding("Jaina", 1)
	check_binding("Theron", 2)
	check(fixture.portrait_state("Valeera").is_empty(), "departed member binding freed")
	print("PASS portrait_party_bindings_join_leave_reorder")
	# No appearance available: offline/out-of-interest heads retain member identity.
	check_error(fixture.set_members(["Theron", "Jaina", "Valeera", "Uther"], false, true))
	await wait_for_heads(4)
	check_error(fixture.set_members(["Theron", "Jaina", "Valeera", "Uther"], false, false))
	await wait_for_heads(4)
	var offline: Dictionary = fixture.portrait_state("Valeera")
	check(offline.desaturated, "offline portrait desaturated")
	check(offline.model_shown, "offline portrait retains rendered member")
	var dead: Dictionary = fixture.portrait_state("Uther")
	check((dead.tint as Color).is_equal_approx(Color(0.35, 0.35, 0.35, 1)), "dead Retail portrait tint")
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	check_offline_pixels(image, offline.rect)
	if not output.is_empty():
		check(image.save_png(output) == OK, "save screenshot")
	print("PASS portrait_party_offline_desaturated_retained_head")
	# Four single-member joins followed by four leaves, no freed viewport remains.
	check_error(fixture.set_members([], false, false))
	await settle()
	var names: Array[String] = []
	for name in ["Theron", "Jaina", "Valeera", "Uther"]:
		names.append(name)
		check_error(fixture.set_members(names, false, true))
		await wait_for_heads(names.size())
	for index in range(4):
		names.pop_back()
		check_error(fixture.set_members(names, false, true))
		await settle()
		check(count_views() == names.size(), "leave releases portrait view/viewport")
	check(count_views() == 0, "no hosts leaked after four joins and four leaves")
	print("PASS portrait_party_four_joins_four_leaves_no_viewport_leak")
	check_error(fixture.set_members(["Theron", "Jaina"], false, true))
	await wait_for_heads(2)
	check_error(fixture.set_members(["Theron", "Jaina"], true, true))
	await settle()
	check(count_views() == 0, "switch to compact frees existing renders")
	print("PASS portrait_party_switch_compact_frees_hosts")
	fixture.free()
	await settle()
	check(root.find_children("PortraitViewport", "SubViewport", true, false).is_empty(), "fixture shutdown frees all portrait viewports")
	quit(0)

func check_binding(name: String, index: int) -> void:
	var state: Dictionary = fixture.portrait_state(name)
	check(state.frame == "PartyMemberFrame%dPortrait" % (index + 1), "member follows reordered slot: " + name)
	check(str(state.appearance).begins_with("player " + name + " race"), "member-specific appearance: " + name)

func wait_for_heads(count: int) -> void:
	for attempt in range(1200):
		check_error(fixture.tick())
		await process_frame
		if fixture.ready_heads() == count and count_views() == count:
			await settle()
			return
	check(false, "timeout loading %d concrete member portraits" % count)

func settle() -> void:
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

func count_views() -> int:
	var views := root.find_children("PortraitView", "TextureRect", true, false).size()
	var viewports := root.find_children("PortraitViewport", "SubViewport", true, false).size()
	check(views == viewports, "one viewport per portrait host")
	return views

func check_offline_pixels(image: Image, rect: Rect2) -> void:
	var colored := 0
	var grey := 0
	for y in range(int(rect.position.y) + 5, int(rect.end.y) - 5):
		for x in range(int(rect.position.x) + 5, int(rect.end.x) - 5):
			var c := image.get_pixel(x, y)
			var spread := maxf(c.r, maxf(c.g, c.b)) - minf(c.r, minf(c.g, c.b))
			if spread > 3.0 / 255.0:
				colored += 1
			elif c.r > 0.08:
				grey += 1
	check(colored == 0 and grey > 15, "offline head has visible grey model pixels, not empty/colored slot")

func check_error(error: String) -> void:
	check(error.is_empty(), error)

func check(condition: bool, message: String) -> void:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
		assert(condition, message)
