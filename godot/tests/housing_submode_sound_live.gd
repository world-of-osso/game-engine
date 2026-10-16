extends "res://tests/housing_expert_source_live.gd"

const POINTER_KIT: int = 286137
const SHORTCUT_KIT: int = 304887
const CLIP: String = "6838456"
const NOMINAL_GAIN: float = 0.125892535 * 0.8
var sound_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Submode sound proof requires explicit private5870 paid owner configuration")
		return
	var config: String = OS.get_environment("XDG_CONFIG_HOME")
	if not config.begins_with("/home/osso/.worktrees/"):
		fail("Sound fixture cannot use a player's settings")
		return
	var settings := FileAccess.get_file_as_string(config.path_join("world-of-osso/options_settings.ron"))
	if not settings.contains("master_volume: 1.0") or not settings.contains("effects_volume: 0.8") or not settings.contains("muted: false"):
		fail("Sound proof requires the existing private master1/effects0.8/unmuted preferences")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693:
		fail("Submode sound proof requires the genuine editable paid bookcase")
		return
	if not await configure_expert_bindings(false): return
	# These Expert-only assignments cannot emit the Expert shortcut kit in Basic.
	var inactive_count: int = kit_count(SHORTCUT_KIT)
	await source_key(KEY_F8)
	if kit_count(SHORTCUT_KIT) != inactive_count:
		fail("Inactive Expert binding emitted an Expert shortcut sound")
		return
	if not await enter_expert(): return
	for spec in [["Translate", "TranslateSubmodeButton", KEY_F8], ["Rotate", "RotateSubmodeButton", KEY_F9], ["Scale", "ScaleSubmodeButton", KEY_F10]]:
		if not await prove_pointer_sound(spec): return
		if not await prove_pointer_sound(spec): return
		if not await prove_shortcut_sound(spec): return
	if not await set_sound_mute(true): return
	if not await enter_expert(): return
	if not await prove_shortcut_sound(["Rotate", "RotateSubmodeButton", KEY_F9], true): return
	if not await set_sound_mute(false): return
	if not await enter_expert(): return
	if not await prove_shortcut_sound(["Scale", "ScaleSubmodeButton", KEY_F10]): return
	if not await configure_expert_bindings(true): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Submode sounds changed confirmed paid decor or storage")
		return
	print("PASS: private5870 source Expert submode sound scheduling ", housing_skin)
	client.free()
	quit(0)

func kit_count(kit: int) -> int:
	return client.merchant_state().ui_sounds.count(kit)

func pointer_event(point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
	Input.parse_input_event(event)

func prove_pointer_sound(spec: Array) -> bool:
	var control: Control = housing_control(spec[1])
	if control == null:
		fail("Missing actual Expert submode button: " + spec[1])
		return false
	var point: Vector2 = control.get_global_rect().get_center()
	push_mouse_motion(point)
	await create_timer(0.4).timeout
	var before: int = kit_count(POINTER_KIT)
	pointer_event(point, true)
	await frames(2)
	if kit_count(POINTER_KIT) != before:
		fail("Expert pointer sound played on press instead of successful release")
		return false
	var generic := client.get_node_or_null("NativeSound/Effects") as AudioStreamPlayer
	if generic != null and generic.is_playing():
		fail("Generic pointer-down click substituted for the source Expert submode sound")
		return false
	pointer_event(point, false)
	if not await wait_housing(func(): return kit_count(POINTER_KIT) == before + 1, "source Expert pointer sound after release", 3000): return false
	if client.housing_state().precision_submode != spec[0]:
		fail("Sound action did not select its actual Expert submode")
		return false
	if not record_sound("pointer-" + spec[0], NOMINAL_GAIN): return false
	await frames(3)
	if kit_count(POINTER_KIT) != before + 1:
		fail("Hover/frame refresh duplicated the Expert pointer kit")
		return false
	return true

func prove_shortcut_sound(spec: Array, muted: bool = false) -> bool:
	var before: int = kit_count(SHORTCUT_KIT)
	var event := InputEventKey.new()
	event.keycode = spec[2]
	event.physical_keycode = spec[2]
	event.pressed = true
	Input.parse_input_event(event)
	if not await wait_housing(func(): return kit_count(SHORTCUT_KIT) == before + 1, "source assigned Expert shortcut kit", 3000): return false
	if not record_sound("key-" + spec[0] + ("-muted" if muted else ""), 0.0 if muted else NOMINAL_GAIN): return false
	event.echo = true
	Input.parse_input_event(event)
	await frames(2)
	event.echo = false
	event.pressed = false
	Input.parse_input_event(event)
	await frames(2)
	if kit_count(SHORTCUT_KIT) != before + 1 or client.housing_state().precision_submode != spec[0]:
		fail("Expert shortcut echo/release duplicated sound or changed submode")
		return false
	return true

func record_sound(phase: String, gain: float) -> bool:
	var sound_root := client.get_node_or_null("NativeSound/UiSoundKits")
	if sound_root == null or sound_root.get_child_count() == 0:
		fail("No real UiSoundKits stream player after source action")
		return false
	var player := sound_root.get_child(sound_root.get_child_count() - 1) as AudioStreamPlayer
	if player == null or player.stream == null or player.stream.resource_name != CLIP or absf(player.volume_linear - gain) > 0.00001 or not is_equal_approx(player.pitch_scale, 1.0):
		fail("Wrong Expert source stream, nominal master/effects/mute gain or zero-pitch playback")
		return false
	if absf(player.stream.get_length() - 0.186250) > 0.002:
		fail("Native source stream length does not match authenticated Ogg")
		return false
	sound_receipts.append({"phase": phase, "skin": housing_skin, "kits": client.merchant_state().ui_sounds, "stream": player.stream.resource_name, "gain": player.volume_linear, "pitch": player.pitch_scale, "length": player.stream.get_length(), "playing": player.playing, "housing": client.housing_state()})
	var output := FileAccess.open(shots.path_join("submode-sound-receipts.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot save actual Expert sound receipts")
		return false
	output.store_string(JSON.stringify(sound_receipts, "\t"))
	return true

func set_sound_mute(muted: bool) -> bool:
	if not await open_housing_binding_page(): return false
	if not await click_exposed_control(menu_control("OptionsTabsound")): return false
	var name := "ToggleSwitchmutedRightHit" if muted else "ToggleSwitchmutedLeftHit"
	if not await wait_housing(func(): return menu_control(name) != null, "native sound mute preference"): return false
	if not await click_exposed_control(menu_control(name)): return false
	return await close_binding_page()
