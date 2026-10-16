extends "res://tests/adventure_instance_loot.gd"
# Physical source list, independent filters and real catalog icons; no item injection.

func first_set_item() -> String:
	var list := control("EncounterJournalContent").get_global_rect()
	for node in client.find_children("JournalSet*Item*", "Button", true, false):
		if str(node.name).ends_with("Icon") or str(node.name).ends_with("Border"):
			continue
		if node.is_visible_in_tree() and list.has_point(node.get_global_rect().get_center()):
			return str(node.name)
	return ""

func choose_class(id: int) -> bool:
	if not await click("JournalClassFilter") or not await click("JournalClassMenu") or not await click("JournalClass%s" % id):
		return false
	if client.adventure_guide_state().get("class", -1) != id:
		stop("Item Sets source class choice did not apply")
		return false
	return true

func choose_spec(id: int) -> bool:
	if not await click("JournalClassFilter") or not await click("JournalSpec%s" % id):
		return false
	if client.adventure_guide_state().get("spec", -1) != id:
		stop("Item Sets source spec choice did not apply")
		return false
	return true

func verify_list_geometry() -> bool:
	var pane := control("LootJournalItemSetsFrame").get_global_rect()
	var list := control("EncounterJournalContent").get_global_rect()
	var first := first_set_item()
	if first.is_empty() or pane.size.distance_to(Vector2(762,332)) > 1.0 or list != pane:
		stop("Source Item Sets viewport geometry missing")
		return false
	var button := control(first).get_global_rect()
	var row := control(first.substr(0,first.find("Item")) + "Row").get_global_rect()
	if row.size.distance_to(Vector2(744,41)) > 1.0 or absf(row.position.x-pane.position.x-10.0)>1.0 or absf(row.position.y-pane.position.y-29.0)>1.0 or button.size.distance_to(Vector2(32,32))>1.0:
		stop("Source first Item Sets row/button geometry mismatch")
		return false
	return verify_text_layout(first.substr(0,first.find("Item")))

func reveal_set_item(name: String) -> bool:
	var set_id := int(name.trim_prefix("JournalSet").get_slice("Item", 0))
	var ids: PackedInt32Array = client.adventure_guide_state().get("item_set_ids", PackedInt32Array())
	var index := ids.find(set_id)
	if index < 0 or ids.size() * 54 + 16 <= 332:
		stop("Actual filtered source list does not contain target or second-page coverage")
		return false
	# Physical thumb input to the offset of the actual observed ordered ID, never UI-state mutation.
	var fraction := clampf(float(index * 54) / float(ids.size() * 54 + 16 - 332), 0.0, 1.0)
	var thumb := control("EncounterJournalContentScrollThumb").get_global_rect()
	var track := control("EncounterJournalContentScrollTrack").get_global_rect()
	if not await move_to("EncounterJournalContentScrollThumb"):
		return false
	await pointer_button(thumb.get_center(), true)
	var point := Vector2(thumb.get_center().x, track.position.y + fraction * (track.size.y-thumb.size.y) + thumb.size.y/2.0)
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion,true)
	await settle()
	await pointer_button(point,false)
	await settle()
	var item := control(name)
	if item == null or not item.is_visible_in_tree() or not control("EncounterJournalContent").get_global_rect().has_point(item.get_global_rect().get_center()):
		stop("Physical source thumb did not reveal authored set%s at observed index%s" % [set_id,index])
		return false
	print("ITEM_SETS_SCROLL actual_id=%s index=%s count=%s fraction=%s" % [set_id,index,ids.size(),fraction])
	return true

func verify_text_layout(name: String) -> bool:
	var title := control(name) as Label
	var level := control(name + "Level") as Label
	if title == null or level == null or title.get_theme_font_size("font_size") != 12 or level.get_theme_font_size("font_size") != 10 or title.get_line_count() != 1 or level.get_line_count() != 1:
		stop("Source set title/level font or single-line layout mismatch")
		return false
	var row := control(name + "Row").get_global_rect()
	print("ITEM_SETS_TEXT_ANCHOR name=%s title_left=%s expected=%s level_left=%s" % [name,title.get_global_rect().position.x,row.position.x+39.0,level.get_global_rect().position.x])
	if absf(title.get_global_rect().position.x-row.position.x-39.0)>1.0 or absf(level.get_global_rect().position.x-title.get_global_rect().position.x)>1.0:
		stop("Source title left39 is centered inside the550px level column")
		return false
	if absf(level.get_global_rect().position.y-title.get_global_rect().end.y-2.0)>1.0:
		stop("Source item level is not anchored below the actual title plus2")
		return false
	print("ITEM_SETS_TEXT title=%s name_h=%s level_h=%s gap=%s" % [name,title.size.y,level.size.y,level.get_global_rect().position.y-title.get_global_rect().end.y])
	return true

func type_search(value: String) -> bool:
	if not await click("EncounterJournalSearchBox"):
		return false
	for character in value:
		var key := InputEventKey.new()
		key.unicode = character.unicode_at(0)
		key.pressed = true
		root.push_input(key,true)
		await process_frame
	await settle()
	return true

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_adventure4") or shots.is_empty():
		stop("Explicit private realm/account and screenshot directory required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server,account,OS.get_environment("ADVENTURE_GUIDE_PASSWORD"),false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	if client.account_state().ui_skin != OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN"):
		stop("Actual account UI skin differs from receipt")
		return
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while (not client.adventure_guide_state().loaded or not client.adventure_guide_state().journal_item_catalog_ready) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not await click("EncounterJournalLootJournalTab") or not await choose_class(1):
		return
	if control("JournalSpecFilter") != null or not verify_list_geometry() or not await shot("itemsets-source-list"):
		return
	if not await click("JournalClassFilter") or control("JournalAllSpecs") != null or not await shot("itemsets-class-spec-menu"):
		return
	if not await click("JournalClassMenu") or not await shot("itemsets-class-submenu") or not await click("JournalClass1"):
		return
	if not await reveal_set_item("JournalSet567Item24544") or not verify_text_layout("JournalSet567") or not await verify_painted_icon("JournalSet567Item24544Icon"):
		return
	if not await move_to("JournalSet567Item24544") or client.tooltip_state().get("title", "") != "Gladiator's Plate Chestpiece":
		stop("Real source set item tooltip missing")
		return
	if not await shot("itemsets-scrolled-resolved-icon-tooltip"):
		return
	if not await choose_class(2) or not await choose_spec(70) or not verify_list_geometry() or not await shot("itemsets-paladin-spec-reset"):
		return
	var count: int = client.adventure_guide_state().item_set_count
	if not await type_search("Lucifron"):
		return
	if client.adventure_guide_state().item_set_count != count or control("EncounterJournalSearchResults") == null:
		stop("Global journal search changed the source Item Sets list")
		return
	if not await shot("itemsets-global-search") or not await click("EncounterJournalSearchBoxClearButton") or not await choose_class(0):
		return
	if not await click("EncounterJournalDungeonTab") or not await click("EncounterJournalLootJournalTab"):
		return
	if client.adventure_guide_state().get("class", -1) != 0 or label_text("JournalClassFilterText") != "All Classes" or not control("JournalClassFilterText").is_visible_in_tree():
		stop("All Classes did not survive reopen or caption restoration")
		return
	if not await shot("itemsets-all-classes-reopened"):
		return
	print("ITEM_SETS_PASS skin=%s sourcepane/rows/menu/realicon/tooltip/scroll/specreset/independentsearch/allclasses" % client.account_state().ui_skin)
	quit(0)
