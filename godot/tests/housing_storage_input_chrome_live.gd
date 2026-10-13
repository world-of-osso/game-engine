extends "res://tests/housing_storage_search_threshold_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Input chrome needs explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false) or housing_count(2330,"stored") != 1:
		fail("Input chrome requires actual owned house/paid Tesla")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "chrome drawer entry"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSearchBox") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "settled chrome input"): return
	var before: Dictionary = client.housing_state()
	for long_text: String in ["W".repeat(45),"é".repeat(45)]:
		if not await type_capped_search(long_text): return
	if not await click_clear(): return
	var input: LineEdit = housing_control("HouseEditorStorageSearchBox") as LineEdit
	var style: StyleBox = input.get_theme_stylebox("normal")
	if absf(style.get_content_margin(SIDE_LEFT)-16.0)>0.01 or absf(style.get_content_margin(SIDE_RIGHT)-20.0)>0.01:
		fail("Storage search missing source16/20 text gutters")
		return
	var glyph: Control = housing_control("HouseEditorStorageSearchGlyph")
	var image: TextureRect = glyph.get_node_or_null("Parts/Part0") as TextureRect if glyph != null else null
	if image == null or image.texture == null:
		fail("Source storage magnifier atlas missing")
		return
	var scale: float = input.get_global_rect().size.y/30.0
	var rect: Rect2 = glyph.get_global_rect()
	var input_rect: Rect2 = input.get_global_rect()
	if rect.size.distance_to(Vector2(10.0,10.0)*scale)>0.2 or absf(rect.position.x-input_rect.position.x-scale)>0.2 or absf(rect.get_center().y-input_rect.get_center().y-scale)>0.2:
		fail("Storage magnifier source rectangle mismatch: "+str(rect))
		return
	if not magnifier_color_matches(image.self_modulate,0.6):
		fail("Unfocused empty source magnifier is notRGB0.6: "+str(image.self_modulate))
		return
	var normal_pixels: Image = await clear_pixels(glyph)
	if not await click_exposed_control(input): return
	await frames(4)
	if not magnifier_color_matches(image.self_modulate,1.0):
		fail("Focused source magnifier is notRGB1: "+str(image.self_modulate))
		return
	var focused_pixels: Image = await clear_pixels(glyph)
	var brighter: int = brighter_clear_pixels(normal_pixels,focused_pixels)
	if brighter==0:
		fail("Source focus tint changed without brighter magnifier pixels")
		return
	print("STORAGE_INPUT_CHROME cap=40 gutters=16,20 brighter_focus=",brighter," glyph_rect=",rect)
	if not await record_storage_tooltip("storage-search-input-focused"): return
	if not await type_capped_search("é".repeat(45)): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCollapseButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null and absf(housing_control("HouseEditorStorageExpandButton").global_position.x)<0.01, "chrome collapse"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSearchBox") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "chrome remount"): return
	input = housing_control("HouseEditorStorageSearchBox") as LineEdit
	glyph = housing_control("HouseEditorStorageSearchGlyph")
	image = glyph.get_node_or_null("Parts/Part0") as TextureRect
	if input.has_focus() or input.text!="é".repeat(40) or input.max_length!=40 or not magnifier_color_matches(image.self_modulate,1.0):
		fail("Nonempty remount lost source cap/UTF8 text/bright magnifier")
		return
	if not await record_storage_tooltip("storage-search-input-utf8-cap"): return
	if not await click_clear(): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null, "clear capped query restores actual paid result"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 owned-storage input chrome ",housing_skin)
	client.free()
	quit(0)

func magnifier_color_matches(color: Color, channel: float) -> bool:
	var rgb: Vector3 = Vector3(color.r,color.g,color.b)
	return rgb.distance_to(Vector3.ONE*channel)<0.02 and absf(color.a-1.0)<0.02

func type_capped_search(long_text: String) -> bool:
	var input: LineEdit = housing_control("HouseEditorStorageSearchBox") as LineEdit
	if not await click_exposed_control(input): return false
	input.select_all()
	await housing_type_text(long_text)
	await frames(6)
	var expected: String = long_text.left(40)
	if input.text!=expected or input.max_length!=40:
		fail("Source40-letter input expected40 characters, actual="+str(input.text.length())+" max_length="+str(input.max_length))
		return false
	print("STORAGE_INPUT_LIMIT chars=",input.text.length()," utf8_bytes=",input.text.to_utf8_buffer().size())
	return true
