extends "res://tests/housing_storage_anchor_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Storage tile proof needs private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693 or housing_count(2330,"stored")!=1:
		fail("Source tile proof needs actual paid bookcase/Tesla fixture")
		return
	var tile: BaseButton = housing_control("HouseEditorStored2330") as BaseButton
	var icon: Control = housing_control("HouseEditorIcon2330")
	var count: Label = housing_control("HouseEditorStored2330Count") as Label
	var scale: float = tile.get_global_rect().size.x/97.0
	if absf(tile.get_global_rect().size.y-97.0*scale)>0.1 or icon==null or count==null:
		fail("Source storage tile is not97x97 with inset icon/count")
		return
	if icon.get_global_rect().size.distance_to(Vector2(87,87)*scale)>0.1 or icon.get_global_rect().position.distance_to(tile.get_global_rect().position+Vector2(5,5)*scale)>0.1:
		fail("Source storage icon is not87x87 at5,5")
		return
	if count.text!="1" or absf(tile.get_global_rect().end.x-count.get_global_rect().end.x-9.0*scale)>0.1 or absf(tile.get_global_rect().end.y-count.get_global_rect().end.y-7.0*scale)>0.1:
		fail("Actual paid count lost bottom-right source anchor")
		return
	var bg_name: String = "HouseEditorStored2330Background"
	var hover_name: String = "HouseEditorStored2330HoverBackground"
	push_mouse_motion(Vector2(500,500))
	if not await wait_housing(func(): return tile_atlas(bg_name)!=null and housing_control(hover_name).modulate.a==0.0, "normal source tile art"): return
	var normal_region: Rect2 = tile_atlas(bg_name).region
	if normal_region.size!=Vector2(192,196) or tile_atlas(bg_name).get_image().get_size()!=Vector2i(192,196):
		fail("Native source atlas member was cropped twice: " + str(normal_region))
		return
	print("TILE_SOURCE_NORMAL region=",normal_region," image=",tile_atlas(bg_name).get_image().get_size()," rect=",housing_control(bg_name).get_global_rect()," modulate=",housing_control(bg_name).modulate)
	tile_atlas(bg_name).get_image().save_png(shots.path_join("tile-source-normal.png"))
	if not await record_editor("storage-tile-normal"): return
	push_mouse_motion(tile.get_global_rect().get_center())
	if not await wait_housing(func(): return absf(housing_control(hover_name).modulate.a-0.75)<0.001, "source additive75percent hover"): return
	if not await record_editor("storage-tile-hover"): return
	await tile_mouse_button(tile.get_global_rect().get_center(),true)
	if not await wait_housing(func(): return tile_atlas(bg_name).region!=normal_region and tile_atlas(bg_name).region==tile_atlas(hover_name).region, "matching source pressed background/hover atlas"): return
	print("TILE_SOURCE_PRESSED region=",tile_atlas(bg_name).region," rect=",housing_control(bg_name).get_global_rect()," modulate=",housing_control(bg_name).modulate)
	tile_atlas(bg_name).get_image().save_png(shots.path_join("tile-source-pressed.png"))
	if not await record_editor("storage-tile-pressed"): return
	if client.housing_state().stored!=before.stored or client.housing_state().decor!=before.decor:
		fail("Tile hold changed paid storage/layout")
		return
	await tile_mouse_button(tile.get_global_rect().get_center(),false)
	if not await wait_housing(func(): return client.housing_state().get("placing",0)==2330 and housing_control("HouseEditorStorageFrame")==null, "normal released tile begins owned placement"): return
	await press_editor_escape()
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330")!=null and client.housing_state().get("placing",0)==0, "normal Cancel restores real tile"): return
	push_mouse_motion(Vector2(500,500))
	if not await wait_housing(func(): return tile_atlas(bg_name).region==normal_region and housing_control(hover_name).modulate.a==0.0, "released source default artwork"): return
	if client.housing_state().stored!=before.stored or client.housing_state().placed!=before.placed or client.housing_state().decor!=before.decor:
		fail("Tile placement/cancel lost or fabricated owned copies")
		return
	if not await record_editor("storage-tile-restored"): return
	print("PASS: private5870 source storage97 tiles ",housing_skin)
	client.free()
	quit(0)

func tile_atlas(name: String) -> AtlasTexture:
	var control: Control = housing_control(name)
	if control==null: return null
	for node in control.find_children("*","TextureRect",true,false):
		if node.texture is AtlasTexture: return node.texture
	return null

func tile_mouse_button(point: Vector2,pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
	Input.parse_input_event(event)
	await frames(3)
