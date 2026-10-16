extends "res://tests/toy_objects_live.gd"
# Owned fb_* accounts, private5911 only. Actual bag learning and Toy Box activation.
var failed := false
const CASES := [
	[64646, 91760, 32473, "Bones of Transformation"],
	[54651, 75531, 31654, "Gnomeregan Pride"],
]

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "assets", 180000): return
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5911": fail("private5911 required"); return
	var account := OS.get_environment("TOYFX_ACCOUNT")
	if not account.begins_with("fb_"): fail("private fb_* account required"); return
	var error: String = client.connect_account(endpoint, account, "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null, "roster", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("CollectionsMicroButton") != null, "world", 180000): return
	var wanted_skin := "Modern" if skin == "modern" else "Forever"
	if client.account_state().ui_skin != wanted_skin: fail("wrong account skin"); return
	if not await wait_until(func(): return native_display(-1), "native player model", 120000): return
	if not await wait_until(func(): return client.merchant_state().item_catalog_loaded, "source item catalog", 180000): return
	if not await wait_until(func(): return client.toybox_state().catalog_count == 1166, "full authenticated toy catalog", 120000): return
	await capture("native")
	for sample in proof_cases():
		await prove_toy(sample)
		if failed: return
	write_receipts()
	print("PASS toy-effects14 actual learn/use/native effect/restoration ", skin)
	client.free()
	quit(0)

func proof_cases() -> Array:
	return CASES

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	var display := int(sample[2])
	await learn_toy(item)
	if failed or not learned_toy(item): return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "search " + str(item)): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return has_aura(spell) and native_display(display), "source transform " + str(display), 120000): return
	await capture("toy%d-display%d" % [item, display])
	if failed: return
	await cancel_aura(spell)
	if failed: return
	if not await wait_until(func(): return not has_aura(spell) and native_display(-1), "native restoration", 120000): return
	await capture("toy%d-restored" % item)

func native_display(expected: int) -> bool:
	var state = client.account_state()
	if state.local_player_id == null: return false
	var model = client.unit_display(int(state.local_player_id))
	if model.is_empty() or int(model.display_id) != expected or not model.visual: return false
	# unit_display reflects the requested appearance while the old model may still be visible.
	for unit in client.find_children(character, "Node3D", true, false):
		var visual := unit.get_node_or_null("PlayerModel" if expected < 0 else "NpcVisualRoot")
		if visual == null or visual.find_children("*", "MeshInstance3D", true, false).is_empty(): continue
		if expected < 0: return true
		if int(visual.get_meta("creature_display_id")) == expected: return true
	return false

func capture(label: String) -> void:
	await capture_native_frame(label)

func capture_native_frame(label: String) -> void:
	await create_timer(1.0).timeout
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-%s.png" % [skin, label])
	if image.save_png(path) != OK: fail("PNG capture"); return
	var state = client.account_state()
	receipts.append({"phase": label, "skin": state.ui_skin, "account": state,
		"model": client.unit_display(int(state.local_player_id)), "auras": client.aura_state(),
		"visuals": client.spell_visuals_state(),
		"toys": client.toybox_state(), "image": path})
	write_receipts()

func fail(message: String) -> void:
	failed = true
	push_error("toy-effects14: " + message)
	if client != null: client.free()
	quit(1)
