extends "res://tests/toy_live2.gd"
# Fresh Orc account with Worn Cloak165791 and Filmless Camera221964 in its bags.
# No Toy Box reopen/query between bag learning and the collection-state assertion.
func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "assets", 180000): return
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("TOYFX_ACCOUNT"), "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null, "roster", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("CollectionsMicroButton") != null, "world", 180000): return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return client.toybox_state().catalog_count > 0, "initial catalog"): return
	if learned_toy(165791) or learned_toy(221964): fail("requires unlearned fixture toys"); return
	await click("CollectionsMicroButton")
	await learn_without_query(165791)
	if failed: return
	if client.toybox_state().open: fail("closed learning opened journal"); return
	receipts.append({"phase": "closed-learning-notification", "toybox": client.toybox_state()})
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await show_learned_tile("Worn Cloak")
	if failed: return
	await learn_without_query(221964)
	if failed: return
	if not client.toybox_state().open: fail("open journal closed on learning"); return
	await show_learned_tile("Filmless Camera")
	if failed: return
	receipts.append({"phase": "open-learning-notification", "toybox": client.toybox_state()})
	error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("TOYFX_NEXT_ACCOUNT"), "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return client.account_state().screen == "CharacterSelect" and control("CharCard_0") != null, "next account roster"): return
	if client.toybox_state().catalog_count != 0 or not client.toybox_state().learned.is_empty(): fail("previous account collection survived connection reset"); return
	receipts.append({"phase": "account-reset", "toybox": client.toybox_state()})
	write_receipts()
	print("PASS: toy learning updates closed collection and open/next-opened Toy Box without reopen workaround ", skin)
	client.free()
	quit(0)

func learn_without_query(item: int) -> void:
	await click("MainMenuBarBackpackButton")
	if not await wait_until(func(): return control("ContainerFrame0") != null, "backpack"): return
	var slot_name := ""
	for bag_item in client.merchant_state().bags:
		if int(bag_item.item_id) == item: slot_name = "ContainerFrame%dSlot%d" % [bag_item.bag, bag_item.slot]
	if slot_name == "": fail("missing fixture toy " + str(item)); return
	print("TOYLEARN_INPUT item=", item, " slot=", slot_name)
	await click(slot_name, MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return learned_toy(item), "learning notification without query " + str(item), 15000): return
	await click("MainMenuBarBackpackButton")

func show_learned_tile(title: String) -> void:
	await click("ToyBoxSearchBox")
	var editor := control("ToyBoxSearchBox") as LineEdit
	editor.select_all()
	await type_text(title)
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == title, "learned tile " + title): return
	if bool(control("ToySpellButton1").get("disabled")): fail("learned toy tile disabled " + title); return
	await RenderingServer.frame_post_draw
	var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-learning-%s.png" % [skin, title.replace(" ", "-")])
	if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
	receipts.append({"phase": "learned-tile", "image": path, "toybox": client.toybox_state()})
