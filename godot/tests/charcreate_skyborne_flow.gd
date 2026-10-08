extends SceneTree
## Offline rendered acceptance. Run scripts/tests/charcreate_skyborne.py.
## Catalog expectations come from imported Forever CSVs, not native implementation.

const SIZE := Vector2i(1280, 720)
const BYTE := 1.0 / 255.0
const ErrorObserver = preload("res://tests/bloom_lifecycle_pixels.gd").ErrorObserver

var client: Node
var ui: Node
var failed := false
var failures := 0
var passed := 0
var observer := ErrorObserver.new()
var directory: String
var data_root: String


func _initialize() -> void:
	OS.add_logger(observer)
	call_deferred("run")


func expect(condition: bool, message: String) -> bool:
	if not condition:
		failed = true
		failures += 1
		printerr("SKYBORNE FAIL: ", message)
	return condition


func rows(table: String) -> Array[Dictionary]:
	var file := FileAccess.open(
		data_root.path_join("db2/1.60.1.70205/" + table + ".csv"), FileAccess.READ
	)
	if not expect(file != null, "Missing Forever table " + table):
		return []
	var headers := file.get_csv_line()
	var result: Array[Dictionary] = []
	while not file.eof_reached():
		var fields := file.get_csv_line()
		if fields.size() != headers.size():
			continue
		var row := {}
		for index in range(headers.size()):
			row[headers[index]] = fields[index]
		result.append(row)
	return result


func press(name: String) -> bool:
	var button := ui.find_child(name, true, false) as Button
	if not expect(
		button != null and button.is_visible_in_tree() and not button.disabled,
		"Missing/enabled button " + name
	):
		return false
	button.pressed.emit()
	for frame in range(4):
		await process_frame
	return true


func shown(name: String) -> bool:
	var node := ui.find_child(name, true, false) as CanvasItem
	return node != null and node.is_visible_in_tree()


func run() -> void:
	directory = OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	data_root = ProjectSettings.globalize_path("res://../data")
	root.size = SIZE
	if not expect(
		directory.contains("/data/diagnostics/skyborne-charcreate"),
		"Require owned capture directory"
	):
		quit(1)
		return
	if not expect(
		DisplayServer.get_name() != "headless" and RenderingServer.get_rendering_device() != null,
		"Require offscreen rendered Vulkan, not --headless"
	):
		quit(1)
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if await wait_for_creation():
		for race in [95, 96]:
			await press("Race_%d" % race)
			for sex in [0, 1]:
				await check_variant(race, sex)
	Engine.time_scale = 1.0
	client.queue_free()
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw
	expect(not is_instance_valid(client), "Client freed before normal shutdown")
	expect(observer.count() == 0, "Engine errors: %d" % observer.count())
	OS.remove_logger(observer)
	print(
		(
			"SKYBORNE RESULT variants=%d/4 failures=%s engine_errors=%d"
			% [passed, failed, observer.count()]
		)
	)
	quit(1 if failed or passed != 4 else 0)


func wait_for_creation() -> bool:
	# Native startup polls an asset worker before initialize_startup mounts screens.
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		ui = client.get_node_or_null("CharacterCreateUI")
		if (
			ui != null
			and client.account_state().screen == "CharacterCreate"
			and client.get_node_or_null("CharacterCreateScene/Camera") != null
			and client.get_node_or_null("CharacterCreateScene/CreationCharacter") != null
		):
			return true
	return expect(false, "Creation startup timed out: " + str(client.account_state()))


func check_variant(race: int, sex: int) -> void:
	var previous_failures := failures
	Engine.time_scale = 1.0
	await press("CharCreateSex_%d" % sex)
	var character := client.get_node_or_null("CharacterCreateScene/CreationCharacter") as Node3D
	var fdid := 7478487 if sex == 0 else 7478494
	var model_id := 218 + sex
	if not expect(character != null, "Missing preview race=%d sex=%d" % [race, sex]):
		return
	expect(
		str(character.get_meta("m2_source_path", "")).get_file() == "%d.m2" % fdid,
		"Body FDID %d" % fdid
	)
	expect(
		client.get_node_or_null("CharacterCreateScene/CharCreateBackdrop_8035354") != null,
		"Authored Skyborne creation backdrop 8035354"
	)
	expect(
		shown("Race_%d_Selected" % race) and shown("CharCreateSex_%d_Selected" % sex),
		"Race/sex selection rings"
	)
	check_classes(race)
	check_icon(race)
	check_materials(character, model_id)
	await check_animation(character)
	await press("CharCreateNext")
	await check_options(model_id, sex)
	await press("CharCreateBack")
	var state: Dictionary = client.account_state()
	expect(
		not state.reply_received and state.character_count == 0 and state.unit_count == 0,
		"No server/authenticated world"
	)
	Engine.time_scale = 0.0
	var stem := "race-%d-sex-%d" % [race, sex]
	var visible := await capture(stem + ".png")
	var control := await capture(stem + "-control.png")
	character.hide()
	var hidden := await capture(stem + "-empty.png")
	character.show()
	var restored := await capture(stem + "-restored.png")
	check_pixels(visible, control, hidden, restored, race, sex)
	Engine.time_scale = 1.0
	if failures == previous_failures:
		passed += 1
		print(
			(
				"SKYBORNE PASS race=%d sex=%d body=%d layout=%d offered_options=%d"
				% [
					race,
					sex,
					fdid,
					201 + sex,
					client.character_creation_catalog().offered_option_ids.size()
				]
			)
		)


func check_classes(race: int) -> void:
	var expected := [1, 3, 4, 8, 11] if race == 95 else [1, 3, 4, 7, 11]
	var labels := (
		["Warrior", "Hunter", "Rogue", "Mage", "Druid"]
		if race == 95
		else ["Warrior", "Hunter", "Rogue", "Shaman", "Druid"]
	)
	var available := []
	var actual_labels := []
	var selected := []
	for id in range(1, 14):
		var button := ui.find_child("Class_%d" % id, true, false) as Button
		if button != null and button.is_visible_in_tree() and not button.disabled:
			available.append(id)
			actual_labels.append((ui.find_child("Class_%d_Label" % id, true, false) as Label).text)
		if shown("Class_%d_Selected" % id):
			selected.append(id)
	expect(
		available == expected and actual_labels == labels,
		"Allowed classes: %s / %s" % [available, actual_labels]
	)
	expect(selected == [8 if race == 95 else 7], "Default class selected: " + str(selected))


func check_icon(race: int) -> void:
	var frame := ui.find_child("Race_%d_Icon" % race, true, false)
	var icon := frame.get_node_or_null("Parts/Part0") as TextureRect if frame != null else null
	if not expect(icon != null and icon.texture != null, "Native race icon texture"):
		return
	var actual := icon.texture.get_image()
	var atlas := Image.load_from_file(directory.path_join("atlas-8200220.png"))
	if not expect(
		actual != null and atlas != null and actual.get_size() == Vector2i(64, 64),
		"64x64 masked atlas crop"
	):
		return
	var compared := 0
	var mismatches := 0
	for y in range(64):
		for x in range(64):
			var pixel := actual.get_pixel(x, y)
			if pixel.a < 0.99:
				continue
			compared += 1
			if contrast(pixel, atlas.get_pixel(x + 1, y + 325)) > 2.0 * BYTE:
				mismatches += 1
	expect(
		compared > 0 and mismatches == 0,
		"FDID 8200220 crop RGB: compared=%d mismatches=%d" % [compared, mismatches]
	)


func check_materials(character: Node3D, model_id: int) -> void:
	var models := rows("ChrModel")
	var layout_id := 0
	for row in models:
		if int(row.ID) == model_id:
			layout_id = int(row.CharComponentTextureLayoutID)
	var dimensions := Vector2i.ZERO
	for row in rows("CharComponentTextureLayouts"):
		if int(row.ID) == layout_id:
			dimensions = Vector2i(int(row.Width), int(row.Height))
	expect(
		layout_id == 201 + model_id - 218 and dimensions == Vector2i(2048, 1024),
		"Catalog body canvas layout/dimensions"
	)
	var visible_geosets := {}
	var materials := 0
	var body_textures := 0
	for child in character.get_children():
		var mesh := child as MeshInstance3D
		if mesh == null or not mesh.visible:
			continue
		expect(
			mesh.mesh != null and mesh.mesh.get_surface_count() > 0, "Visible body mesh surfaces"
		)
		var geoset: int = mesh.get_meta("m2_mesh_part", -1)
		expect(geoset >= 0, "Authored body geoset ID")
		visible_geosets[geoset] = true
		var material := mesh.get_active_material(0) as ShaderMaterial
		if not expect(material != null, "Visible body batch material"):
			continue
		materials += 1
		for slot in ["base_texture", "second_texture", "third_texture", "fourth_texture"]:
			var texture: Variant = material.get_shader_parameter(slot)
			if texture is Texture2D and texture.get_size() == Vector2(dimensions):
				body_textures += 1
				var image: Image = texture.get_image()
				expect(image != null and not image.is_empty(), "Composed body texture pixels")
	expect(
		visible_geosets.size() > 0 and materials > 0 and body_textures > 0,
		"Nonzero visible geosets/materials/composed body textures"
	)
	print(
		(
			"SKYBORNE MATERIALS model=%d geosets=%d materials=%d body_bindings=%d layout=%d size=%s"
			% [model_id, visible_geosets.size(), materials, body_textures, layout_id, dimensions]
		)
	)


func check_animation(character: Node3D) -> void:
	var skeleton := character.get_node_or_null("Skeleton3D") as Skeleton3D
	var animation := character.get_node_or_null("M2Animation")
	if not expect(
		skeleton != null and animation != null and skeleton.get_bone_count() > 0,
		"Bound native skeleton and animation player"
	):
		return
	# MD21 wraps MD20 at byte 8; the inline bone M2Array starts at 0x2c.
	# Assert the live palette against the body, not ChrModel's unrelated MD21 FDID.
	var source := str(character.get_meta("m2_source_path"))
	var file := FileAccess.open(source, FileAccess.READ)
	if not expect(file != null, "Read loaded body header for skeleton oracle"):
		return
	var header := file.get_buffer(64)
	if not expect(
		header.size() == 64 and header.slice(0, 4).get_string_from_ascii() == "MD21",
		"Body MD21 header"
	):
		return
	var authored_bones := header.decode_u32(8 + 0x2c)
	expect(skeleton.get_bone_count() == authored_bones, "Live palette matches inline body bones")
	var poses := []
	for bone in range(skeleton.get_bone_count()):
		poses.append(skeleton.get_bone_pose(bone))
	await create_timer(0.4).timeout
	var changed := 0
	for bone in range(skeleton.get_bone_count()):
		if not poses[bone].is_equal_approx(skeleton.get_bone_pose(bone)):
			changed += 1
	expect(changed > 0, "Idle animation must change actual bone poses")
	print("SKYBORNE ANIMATION bones=%d changed=%d" % [skeleton.get_bone_count(), changed])


func check_options(model_id: int, sex: int) -> void:
	var catalog: Dictionary = client.character_creation_catalog()
	if not expect(not catalog.has("error"), "Active creation catalog query: " + str(catalog)):
		return
	var raw := []
	var npc_eye_style := []
	for row in rows("ChrCustomizationOption"):
		if int(row.ChrModelID) == model_id:
			raw.append(int(row.ID))
			if row.Name_lang == "Eye Style":
				npc_eye_style.append(int(row.ID))
	raw.sort()
	var native_raw := Array(catalog.raw_option_ids)
	native_raw.sort()
	expect(
		raw.size() == 18 + sex and native_raw == raw,
		"Raw catalog model options match imported 18/19"
	)
	var expected := Array(catalog.offered_option_ids)
	expected.sort()
	for id in npc_eye_style:
		expect(not expected.has(id), "Catalog excludes NPC-only Eye Style")
	var categories := []
	for node in ui.find_children("Category_*", "Button", true, false):
		if node.is_visible_in_tree():
			categories.append(str(node.name))
	var seen := {}
	for category in categories:
		await press(category)
		for node in ui.find_children("Option_*", "Control", true, false):
			var suffix := str(node.name).trim_prefix("Option_")
			if suffix.is_valid_int() and node.is_visible_in_tree():
				seen[int(suffix)] = true
	var actual := seen.keys()
	actual.sort()
	expect(
		actual == expected,
		"Native option IDs match offered catalog: actual=%s expected=%s" % [actual, expected]
	)
	for id in npc_eye_style:
		expect(not actual.has(id), "NPC-only Eye Style absent from native UI")
	print(
		(
			"SKYBORNE OPTIONS model=%d raw=%d offered=%d ui=%d"
			% [model_id, raw.size(), expected.size(), actual.size()]
		)
	)


func capture(filename: String) -> Image:
	for frame in range(24):
		await process_frame
		await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if not expect(image != null and image.get_size() == SIZE, "Rendered viewport pixels"):
		return null
	expect(image.save_png(directory.path_join(filename)) == OK, "Save screenshot " + filename)
	return image


func contrast(a: Color, b: Color) -> float:
	return maxf(absf(a.r - b.r), maxf(absf(a.g - b.g), absf(a.b - b.b)))


func check_pixels(
	visible: Image, control: Image, hidden: Image, restored: Image, race: int, sex: int
) -> void:
	if visible == null or control == null or hidden == null or restored == null:
		return
	var attributable := 0
	var varying := 0
	# Central preview region excludes both race columns and class/navigation UI.
	for y in range(100, 600):
		for x in range(300, 980):
			var noise := maxf(
				contrast(visible.get_pixel(x, y), control.get_pixel(x, y)),
				contrast(visible.get_pixel(x, y), restored.get_pixel(x, y))
			)
			var delta := minf(
				contrast(visible.get_pixel(x, y), hidden.get_pixel(x, y)),
				contrast(restored.get_pixel(x, y), hidden.get_pixel(x, y))
			)
			if noise > BYTE:
				varying += 1
			if delta > maxf(2.0 * BYTE, noise * 4.0):
				attributable += 1
	expect(
		attributable > 0 and attributable > varying,
		"Preview-only hide/restore dominates scene variation"
	)
	print(
		(
			"SKYBORNE PIXELS race=%d sex=%d attributable=%d varying=%d"
			% [race, sex, attributable, varying]
		)
	)
