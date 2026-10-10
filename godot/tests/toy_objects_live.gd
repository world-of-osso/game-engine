extends "res://tests/toy_effects_live.gd"
# Actual UseToy on an owned private toy-effects4 realm; no injected game objects.
var skin := OS.get_environment("TOYFX_SKIN")
const TOYS := [
	[45011, 194274, 10483, "Stormwind Banner"],
	[33223, 186475, 7467, "Fishing Chair"],
	[40768, 191605, 8171, "MOLL-E"],
]

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
	if not await wait_until(func():
		var nodes := client.find_children(character, "Node3D", true, false)
		if nodes.is_empty(): return false
		actor = nodes[0]
		return actor.find_children("PlayerModel", "Node3D", true, false).size() > 0, "native player visual", 120000): return
	await create_timer(2.0).timeout
	for toy in TOYS:
		await learn_toy(int(toy[0]))
		if not learned_toy(int(toy[0])): return
		await click("CollectionsMicroButton")
		await click("ToyBoxSearchBox")
		var edit := control("ToyBoxSearchBox") as LineEdit
		edit.select_all()
		await type_text(str(toy[3]))
		if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(toy[3]), "toy search " + str(toy[0])): return
		await click("ToySpellButton1")
		if not await wait_until(func(): return object_node(int(toy[1])) != null, "spawned asset " + str(toy[1]), 45000): return
		await tap(KEY_ESCAPE)
		var object := object_node(int(toy[1]))
		var model := object.get_node("GameObjectModel") as Node3D
		var meshes := model.find_children("*", "MeshInstance3D", true, false)
		if meshes.is_empty(): fail("spawned model contains no meshes"); return
		if int(object.get_meta("game_object_display_id")) != int(toy[2]): fail("wrong replicated display"); return
		if int(toy[0]) == 45011 and not model.find_children("*", "Area3D", true, false).is_empty(): fail("decoration acquired picking area"); return
		# Leave the newly placed prop ahead of the player/camera, not inside the actor.
		var pos := actor.position
		if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(pos.x + 3.0), str(-pos.z), str(pos.y)]): return
		await create_timer(2.0).timeout
		await RenderingServer.frame_post_draw
		var path := OS.get_environment("TOYFX_SHOTS").path_join("%s-toy%d.png" % [skin, int(toy[0])])
		if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
		receipts.append({"skin": skin, "item": int(toy[0]), "entry": int(toy[1]), "display": int(toy[2]), "fdid": int(model.get_meta("model_file_data_id")), "meshes": meshes.size(), "scale": object.scale.x, "screenshot": path})
		write_receipts()
	print("PASS: actual effect50 toy assets ", JSON.stringify(receipts))
	client.free()
	quit(0)

func learned_toy(item: int) -> bool:
	for toy in client.toybox_state().learned:
		if int(toy.item_id) == item: return true
	return false

func learn_toy(item: int) -> void:
	if learned_toy(item): return
	await click("MainMenuBarBackpackButton")
	if not await wait_until(func(): return control("ContainerFrame0") != null, "backpack"): return
	var name := ""
	for bag_item in client.merchant_state().bags:
		if int(bag_item.item_id) == item: name = "ContainerFrame%dSlot%d" % [bag_item.bag, bag_item.slot]
	if name == "": fail("toy absent from private bags " + str(item)); return
	await click(name, MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return learned_toy(item), "learning " + str(item)): return
	await tap(KEY_ESCAPE)

func object_node(entry: int) -> Node3D:
	for node in client.find_children("*", "Node3D", true, false):
		if node.has_meta("game_object_entry") and int(node.get_meta("game_object_entry")) == entry:
			return node
	return null
