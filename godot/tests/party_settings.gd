extends SceneTree

# Offline real-engine settings and portrait proof; no server or GameClient login.
var fixture: Node
var out := OS.get_environment("GODOT_PARTY4_OUTPUT")
var skin := OS.get_environment("GODOT_CAPTURE_SCREEN")

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	fixture = ClassDB.instantiate("PartyPortraitFixture")
	root.add_child(fixture)
	check_error(fixture.initialize(skin.begins_with("forever")))
	var names: Array[String] = ["Theron", "Jaina", "Valeera", "Uther"]
	check_error(fixture.set_members(names, true, true))
	await settle()
	var compact: Rect2 = fixture.member_rect("CompactPartyFrameMember1")
	check(compact.size.x > 0 and compact.size.y > 0, "compact default drawn")
	check(fixture.ready_heads() == 0, "compact creates no portrait renders")
	print("PASS party4_engine_compact_default")
	check_error(fixture.settings_action("options_toggle:party_compact", 0))
	await wait_heads(4)
	check_error(fixture.show_settings(true))
	await settle()
	for name in ["party_compact", "party_background", "party_horizontal", "party_border", "party_pets", "Sliderparty_width", "Sliderparty_height", "Sliderparty_size", "Sliderparty_opacity", "Sliderparty_debuff", "Sliderparty_buff", "Sliderparty_defensive", "PartyDropdownButtonparty_sort", "PartyDropdownButtonparty_aura"]:
		check((fixture.settings_rect(name) as Rect2).size.x > 0, "mounted settings control: " + name)
	var inactive_preset := "Choiceui_layout0Hit" if skin.begins_with("forever") else "Choiceui_layout1Hit"
	check((fixture.settings_rect(inactive_preset) as Rect2).size.x > 0, "settings selector matches rendered skin")
	await scroll_to("party_compact")
	await capture("settings-top")
	await scroll_to("Sliderparty_defensive")
	await capture("settings-bottom")
	print("PASS party4_engine_settings_panel_controls")
	check_error(fixture.settings_action("options_toggle:party_sort_open", 0))
	await settle()
	check((fixture.settings_rect("PartyChoiceparty_sort0") as Rect2).size.x > 0, "sort dropdown expands")
	check_error(fixture.settings_action("options_toggle:party_sort:2", 0))
	await wait_heads(4)
	check((fixture.portrait_state("Jaina") as Dictionary).frame == "PartyMemberFrame1Portrait", "alphabetical sort moves head with name")
	check_error(fixture.settings_action("options_toggle:party_sort:1", 0))
	await wait_heads(4)
	print("PASS party4_engine_sort_rebinds_heads")
	check_error(fixture.show_settings(false))
	await capture("portrait")
	var before: Rect2 = fixture.member_rect("PartyMemberFrame1")
	check_error(fixture.settings_action("options_slider:party_width", 120))
	check_error(fixture.settings_action("options_slider:party_height", 60))
	check_error(fixture.settings_action("options_toggle:party_horizontal", 0))
	check_error(fixture.settings_action("options_toggle:party_background", 0))
	check_error(fixture.settings_action("options_toggle:party_border", 0))
	await wait_heads(4)
	var after: Rect2 = fixture.member_rect("PartyMemberFrame1")
	var second: Rect2 = fixture.member_rect("PartyMemberFrame2")
	check(after.size.x > before.size.x and after.size.y > before.size.y, "size applies to rendered portrait member")
	check(second.position.x > after.position.x and is_equal_approx(second.position.y, after.position.y), "horizontal portrait layout")
	var head: Dictionary = fixture.portrait_state("Theron")
	var host := fixture.find_child("PartyMemberFrame1Portrait", true, false) as Control
	check((head.rect as Rect2).is_equal_approx(host.get_global_rect()), "head occupies resized host")
	await capture("portrait-horizontal")
	print("PASS party4_engine_live_geometry_and_chrome")
	check_error(fixture.settings_action("options_toggle:party_compact", 0))
	await settle()
	check(fixture.ready_heads() == 0, "compact releases heads")
	check_error(fixture.settings_action("options_reset_layout_settings", 0))
	await settle()
	var reset: Rect2 = fixture.member_rect("CompactPartyFrameMember1")
	check(reset.size.is_equal_approx(compact.size), "Reset restores default compact dimensions")
	print("PASS party4_engine_reset_and_portrait_cleanup")
	fixture.free()
	await settle()
	check(root.find_children("*", "SubViewport", true, false).is_empty(), "fixture cleanup leaves no portrait viewports")
	quit(0)

func scroll_to(name: String) -> void:
	var content: Rect2 = fixture.settings_rect("OptionsContentScroll")
	for attempt in range(80):
		var target: Rect2 = fixture.settings_rect(name)
		if target.position.y >= content.position.y + 10 and target.end.y <= content.end.y - 10:
			return
		var wheel := InputEventMouseButton.new()
		wheel.position = content.get_center()
		wheel.global_position = wheel.position
		wheel.button_index = MOUSE_BUTTON_WHEEL_DOWN if target.end.y > content.end.y else MOUSE_BUTTON_WHEEL_UP
		wheel.pressed = true
		Input.parse_input_event(wheel)
		await settle()
	check(false, "settings scroll reaches " + name)

func capture(suffix: String) -> void:
	await settle()
	await RenderingServer.frame_post_draw
	check(root.get_texture().get_image().save_png(out.path_join(skin + "-" + suffix + ".png")) == OK, "save " + suffix)

func wait_heads(count: int) -> void:
	for attempt in range(1200):
		check_error(fixture.tick())
		await process_frame
		if fixture.ready_heads() == count:
			await settle()
			return
	check(false, "portrait loading timeout")

func settle() -> void:
	for index in range(4):
		await process_frame

func check_error(error: String) -> void:
	check(error.is_empty(), error)

func check(ok: bool, message: String) -> void:
	if not ok:
		push_error("FAIL party4: " + message)
		quit(1)
		assert(ok, message)
