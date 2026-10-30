extends SceneTree
# Isolated production projection: display text, actual ink, and dynamic child lifetime.
const PLAIN := "Earth Breakers continually cast [Skeletal Fractures]."
const COLOURED := "Earth Breakers continually cast |cFF2959D3|Hspell:377780|h[Skeletal Fractures]|h|r."
var owner: Node

func _initialize() -> void:
	call_deferred("watchdog")
	call_deferred("run_contract")

func watchdog() -> void:
	await create_timer(90.0).timeout
	require(false, "projection transition exceeded90s")

func settle() -> void:
	for frame in 8:
		await process_frame
	await RenderingServer.frame_post_draw

func text_node() -> Control:
	return owner.find_child("TextMarkupValue", true, false) as Control

func displayed(node: Node) -> String:
	return node.get_parsed_text() if node.has_method("get_parsed_text") else node.text

func ink_counts(rect: Rect2) -> Vector2i:
	var image := root.get_texture().get_image()
	var counts := Vector2i.ZERO
	for y in range(int(rect.position.y), int(rect.end.y)):
		for x in range(int(rect.position.x), int(rect.end.x)):
			var pixel := image.get_pixel(x, y)
			if pixel.b > 0.4 and pixel.b > pixel.r * 1.5 and pixel.b > pixel.g * 1.2:
				counts.x += 1
			if minf(pixel.r, minf(pixel.g, pixel.b)) > 0.75:
				counts.y += 1
	return counts

func require(value: bool, reason: String) -> bool:
	if not value:
		push_error("TEXT_MARKUP_PROJECTION_FAIL " + reason)
		quit(1)
	return value

func run_contract() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	for forever in [true]:
		owner = ClassDB.instantiate("RegistryUi")
		root.add_child(owner)
		if not require(owner.show_text_markup_fixture(forever).is_empty(), "fixture initialization"):
			return
		await settle()
		var bounds := text_node().get_global_rect()
		var child := owner.find_child("TextMarkupChild", true, false) as Control
		var child_id := child.get_instance_id()
		var initial_ink := ink_counts(bounds)
		if not require(displayed(text_node()) == PLAIN and initial_ink.x == 0 and initial_ink.y > 20, "initial plain display and authored white ink"):
			return
		if not require(owner.text_markup_fixture_step(COLOURED).is_empty(), "reactive coloured update"):
			return
		await settle()
		if not require(displayed(text_node()) == PLAIN, "markup must not paint its control bytes"):
			return
		var coloured_ink := ink_counts(text_node().get_global_rect())
		if not require(coloured_ink.x > 20 and coloured_ink.y > 20, "authored blue link and unchanged white prefix ink absent"):
			return
		if not require(text_node().get_global_rect() == bounds and is_instance_valid(child) and child.is_visible_in_tree() and child.get_instance_id() == child_id, "plain-to-coloured update changed bounds or destroyed the child"):
			return
		if not require(owner.text_markup_fixture_step(PLAIN).is_empty(), "reactive plain update"):
			return
		await settle()
		var restored_ink := ink_counts(bounds)
		if not require(displayed(text_node()) == PLAIN and restored_ink.x == 0 and restored_ink.y > 20 and is_instance_valid(child) and child.is_visible_in_tree(), "coloured-to-plain update retained colour or lost child"):
			return
		if not require(owner.text_markup_fixture_step("||cFF2959D3 Kurog || Fractures").is_empty(), "escaped-pipe update"):
			return
		await settle()
		if not require(displayed(text_node()) == "|cFF2959D3 Kurog | Fractures", "escaped pipes became formatting"):
			return
		print("TEXT_MARKUP_PROJECTION_PASS skin=%s display/blue-ink/plain-coloured-plain/child-preserved/escaped-pipes" % forever)
		owner.free()
	quit(0)
