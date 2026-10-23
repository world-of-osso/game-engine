extends SceneTree
# Isolated production projection: display text, actual ink, and dynamic child lifetime.
const PLAIN := "Earth Breakers continually cast [Skeletal Fractures]."
const COLOURED := "Earth Breakers continually cast |cFF2959D3|Hspell:377780|h[Skeletal Fractures]|h|r."
var owner: Node

func _initialize() -> void:
	call_deferred("run_contract")

func settle() -> void:
	for frame in 8:
		await process_frame
	await RenderingServer.frame_post_draw

func text_node() -> Control:
	return owner.find_child("TextMarkupValue", true, false) as Control

func displayed(node: Node) -> String:
	return node.get_parsed_text() if node.has_method("get_parsed_text") else node.text

func blue_pixels(rect: Rect2) -> int:
	var image := root.get_texture().get_image()
	var count := 0
	for y in range(int(rect.position.y), int(rect.position.y + 45.0)):
		for x in range(int(rect.position.x), int(rect.end.x)):
			var pixel := image.get_pixel(x, y)
			if pixel.b > 0.4 and pixel.b > pixel.r * 1.5 and pixel.b > pixel.g * 1.2:
				count += 1
	return count

func require(value: bool, reason: String) -> bool:
	if not value:
		push_error("TEXT_MARKUP_PROJECTION_FAIL " + reason)
		quit(1)
	return value

func run_contract() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	for forever in [false, true]:
		owner = ClassDB.instantiate("RegistryUi")
		root.add_child(owner)
		if not require(owner.show_text_markup_fixture(forever).is_empty(), "fixture initialization"):
			return
		await settle()
		var bounds := text_node().get_global_rect()
		var child := owner.find_child("TextMarkupChild", true, false) as Control
		var child_id := child.get_instance_id()
		if not require(displayed(text_node()) == PLAIN and blue_pixels(bounds) == 0, "initial plain display"):
			return
		if not require(owner.text_markup_fixture_step(COLOURED).is_empty(), "reactive coloured update"):
			return
		await settle()
		if not require(displayed(text_node()) == PLAIN, "markup must not paint its control bytes"):
			return
		if not require(blue_pixels(text_node().get_global_rect()) > 20, "authored blue hyperlink ink absent"):
			return
		if not require(text_node().get_global_rect() == bounds and is_instance_valid(child) and child.is_visible_in_tree() and child.get_instance_id() == child_id, "plain-to-coloured update changed bounds or destroyed the child"):
			return
		if not require(owner.text_markup_fixture_step(PLAIN).is_empty(), "reactive plain update"):
			return
		await settle()
		if not require(displayed(text_node()) == PLAIN and blue_pixels(bounds) == 0 and is_instance_valid(child) and child.is_visible_in_tree(), "coloured-to-plain update retained colour or lost child"):
			return
		if not require(owner.text_markup_fixture_step("||cFF2959D3 Kurog || Fractures").is_empty(), "escaped-pipe update"):
			return
		await settle()
		if not require(displayed(text_node()) == "|cFF2959D3 Kurog | Fractures", "escaped pipes became formatting"):
			return
		print("TEXT_MARKUP_PROJECTION_PASS skin=%s display/blue-ink/plain-coloured-plain/child-preserved/escaped-pipes" % forever)
		owner.free()
	quit(0)
