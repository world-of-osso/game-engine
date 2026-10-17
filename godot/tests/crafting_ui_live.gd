extends "res://tests/bank_live.gd"

## Private joint crafting proof. Uses normal character selection and native physical input.
## GODOT_TEST_SERVER must be 127.0.0.1:5871. BANK_ACCOUNT/BANK_CHARACTER/BANK_SHOTS
## isolate the account and evidence. CRAFTING_PHASE selects spec, customer, crafter,
## customer-mail or crafter-mail; recipe/node/broker/mailbox IDs come from the live fixture.
func run_test() -> void:
	root.size = Vector2i(1280, 900)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5871" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_"):
		fail("Crafting proof requires owned fb_* account on private UDP5871")
		return
	role = OS.get_environment("CRAFTING_PHASE")
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(endpoint, OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	# Client graphics startup overrides the SceneTree's initial cap. Replay human-paced input.
	Engine.max_fps = 60
	print("CRAFTING ACCOUNT ", client.account_state().ui_skin, " ", OS.get_environment("BANK_CHARACTER"))
	var completed := false
	match role:
		"spec": completed = await spend_knowledge()
		"customer": completed = await place_public_order()
		"crafter": completed = await fulfill_public_order()
		"customer-mail", "crafter-mail": completed = await receive_order_mail()
		_: fail("Unknown crafting phase: " + role)
	if not completed:
		return
	print("FIXTURE CRAFTING_UI_LIVE_DONE phase=", role, " skin=", client.account_state().ui_skin)
	client.free()
	quit(0)

const CRAFTING_TRIANGLES := preload("res://tests/posed_triangle_points.gd")

func click_pickable(matches: Callable, id_meta: String) -> bool:
	for turn in range(48):
		var camera := root.get_camera_3d()
		if camera != null:
			var nodes := nearest_first(client.find_children("*", "Node3D", true, false).filter(matches))
			for node in nodes:
				var area: Node = node.find_child("UnitPick", true, false)
				if area == null:
					continue
				var id: int = area.get_meta(id_meta) if area.has_meta(id_meta) else node.get_meta(id_meta)
				var point := pick_mesh_point(node, camera, id)
				if point != Vector2.INF:
					await click_at(point, MOUSE_BUTTON_RIGHT)
					return true
		push_key(KEY_LEFT, 0, true)
		await create_timer(0.2).timeout
		push_key(KEY_LEFT, 0, false)
		await create_timer(0.04).timeout
	await capture("fail-pick")
	return fail("No visible source model triangle is pickable")

func pick_mesh_point(node: Node3D, camera: Camera3D, id: int) -> Vector2:
	for candidate in node.find_children("*", "MeshInstance3D", true, false):
		var mesh := candidate as MeshInstance3D
		if mesh.mesh == null or not mesh.is_visible_in_tree():
			continue
		for surface in range(mesh.mesh.get_surface_count()):
			var centers := CRAFTING_TRIANGLES.centroids(mesh, surface)
			centers.sort_custom(func(a, b): return a.y < b.y)
			for center in centers:
				var point := camera.unproject_position(center)
				if camera.is_position_in_frustum(center) and Rect2(Vector2.ZERO, Vector2(root.size)).has_point(point) and UnitPicker.pick(camera, point) == id and world_pick_point_clear(point):
					return point
	return Vector2.INF

func world_pick_point_clear(point: Vector2) -> bool:
	# A physics hit behind the HUD is not a physical world click (e.g. bag buttons).
	for node in client.find_children("*", "Control", true, false):
		var target := node as Control
		if target.is_visible_in_tree() and target.mouse_filter == Control.MOUSE_FILTER_STOP and target.get_global_rect().has_point(point):
			return false
	return true

func open_recipe_book() -> bool:
	await tap(KEY_K)
	var recipe := "ProfessionRecipe" + OS.get_environment("CRAFTING_OPEN_RECIPE")
	if not await wait_until(func(): return shown(recipe), "learned recipe " + recipe):
		return false
	await press(recipe)
	return true

func spend_knowledge() -> bool:
	if not await open_recipe_book():
		return false
	await press("ProfessionsSpecializationsTab")
	var tree := "ProfessionSpecTree" + OS.get_environment("CRAFTING_TREE")
	if not await wait_until(func(): return shown(tree), "authored specialization tree"):
		return false
	await press(tree)
	if shown("ProfessionSpecUnlock"):
		await press("ProfessionSpecUnlock")
		if not await wait_until(func(): return shown("ProfessionSpecConfirmYes"), "unlock confirmation"):
			return false
		await press("ProfessionSpecConfirmYes")
		if not await wait_until(func(): return shown("ProfessionSpecApply"), "authoritative tree unlock"):
			return false
	await press("ProfessionSpecNode" + OS.get_environment("CRAFTING_NODE"))
	var committed := text("ProfessionSpecDetailedRank")
	var knowledge := text("ProfessionSpecKnowledge")
	await capture("1-committed-tree")
	if not await observe_trait_tooltip("ProfessionSpecNode" + OS.get_environment("CRAFTING_NODE"), "1-path-tooltip"):
		return false
	if not await observe_trait_tooltip("ProfessionSpecPerk" + OS.get_environment("CRAFTING_PERK"), "1-perk-tooltip"):
		return false
	await press("ProfessionSpecSpend")
	if not await wait_until(func(): return text("ProfessionSpecDetailedRank") != committed, "staged knowledge point"):
		return false
	var staged := text("ProfessionSpecDetailedRank")
	await capture("2-staged-point")
	await press("ProfessionSpecUndo")
	if not await wait_until(func(): return text("ProfessionSpecDetailedRank") == committed and text("ProfessionSpecKnowledge") == knowledge, "Undo without committed refund"):
		return false
	await press("ProfessionSpecSpend")
	await press("ProfessionSpecApply")
	if not await wait_until(func(): return text("ProfessionSpecDetailedRank") == staged and control("ProfessionSpecSpend") is Button and not control("ProfessionSpecSpend").disabled and text("ProfessionSpecStatus") != "", "authoritative point commit response"):
		return false
	# After the response, Undo must not erase the newly committed server rank.
	await press("ProfessionSpecUndo")
	if text("ProfessionSpecDetailedRank") != staged:
		return fail("Apply did not commit the staged knowledge rank")
	print("CRAFTING SPEC before=", committed, " after=", staged, " knowledge_before=", knowledge, " knowledge_after=", text("ProfessionSpecKnowledge"), " status=", text("ProfessionSpecStatus"))
	await capture("3-applied-point")
	return true

func observe_trait_tooltip(name: String, shot: String) -> bool:
	if not shown(name):
		return fail("Missing source trait control: " + name)
	var point := control(name).get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	if not await wait_until(func(): return client.tooltip_state().visible, "native trait tooltip ownership"):
		return false
	var tooltip: Dictionary = client.tooltip_state()
	print("CRAFTING TOOLTIP ", name, " ", tooltip)
	if str(tooltip.lines).contains("$ev") or str(tooltip.lines).contains("$en"):
		return fail("Unresolved source profession trait effect tokens")
	await capture(shot)
	return true

func place_public_order() -> bool:
	var broker := OS.get_environment("CRAFTING_BROKER")
	if not await click_pickable(func(node): return str(node.name) == broker, "unit_server_id"):
		return false
	if not await wait_until(func(): return shown("AuctionGossipOption0"), "source customer broker gossip"):
		return false
	await press("AuctionGossipOption0")
	if not await wait_until(func(): return shown("ProfessionsCustomerOrdersFrame"), "authoritative customer service"):
		return false
	await press("CraftingOrdersBrowseTab")
	var recipe := "CraftingOrderRecipe" + OS.get_environment("CRAFTING_ORDER_RECIPE")
	if not await wait_until(func(): return shown(recipe), "source-scoped order recipe"):
		return false
	await press(recipe)
	if not await wait_until(func(): return shown("CraftingOrderPlace"), "customer order form"):
		return false
	await replace_field("CraftingOrderGold", "1")
	await press("CraftingOrderNote")
	await type_text("Joint native public order proof")
	for index in range(OS.get_environment("CRAFTING_SUPPLY_SLOTS").to_int()):
		await press("CraftingOrderSupply%d" % index)
	var before := money()
	await capture("1-public-order-form")
	await press("CraftingOrderPlace")
	if not await wait_until(func(): return money() == before - 10000, "public order commission escrow"):
		return false
	await press("CraftingOrdersMyOrdersTab")
	if not await wait_until(func(): return not client.find_children("CraftingOrderRow*", "Control", true, false).is_empty(), "owned order snapshot"):
		return false
	await click(client.find_children("CraftingOrderRow*", "Control", true, false)[0])
	if not await wait_until(func(): return text("CraftingOrderState") == "Open", "authoritative Open order"):
		return false
	print("CRAFTING ORDER commission=10000 before=", before, " after=", money(), " status=", text("CraftingOrdersStatus"))
	await capture("2-public-order-escrow")
	return true

func fulfill_public_order() -> bool:
	if not await open_recipe_book():
		return false
	await press("ProfessionsOrdersTab")
	if not await wait_until(func(): return not client.find_children("CraftingOrderRow*", "Control", true, false).is_empty(), "public order browse"):
		return false
	await click(client.find_children("CraftingOrderRow*", "Control", true, false)[0])
	if not await wait_until(func(): return shown("CraftingOrderClaim"), "order detail"):
		return false
	await capture("1-public-order-open")
	await press("CraftingOrderClaim")
	if not await wait_until(func(): return text("CraftingOrderState") == "Claimed", "authoritative claim"):
		return false
	await capture("2-public-order-claimed")
	await press("CraftingOrderFulfill")
	if not await wait_until(func(): return text("CraftingOrdersStatus") == "Order completed" and not shown("CraftingOrderState"), "real craft cast completion and removal from open orders"):
		return false
	print("CRAFTING FULFILLED ", text("CraftingOrdersStatus"), " bags=", client.merchant_state().bags)
	await capture("3-public-order-completed")
	return true

func press(name: String) -> void:
	var target := control(name)
	if target == null or not target.is_visible_in_tree():
		fail("Native crafting control is absent: " + name)
		return
	await click(target)

func click_at(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await create_timer(0.03).timeout
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await create_timer(0.05).timeout
	await wait_frames(3)

func replace_field(name: String, value: String) -> void:
	await press(name)
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_A
		event.physical_keycode = KEY_A
		event.ctrl_pressed = true
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await tap(KEY_BACKSPACE)
	await type_text(value)

func receive_order_mail() -> bool:
	var entry := OS.get_environment("CRAFTING_MAILBOX_ENTRY").to_int()
	if not await click_pickable(func(node): return node.has_meta("game_object_entry") and int(node.get_meta("game_object_entry")) == entry, "game_object_server_id"):
		return false
	if not await wait_until(func(): return shown("MailFrame") and client.mail_state().has("mails") and not client.mail_state().mails.is_empty(), "order delivery inbox"):
		return false
	print("CRAFTING MAIL before=", client.mail_state())
	await capture("1-order-mail-inbox")
	await press("MailItem1Button")
	if not await wait_until(func(): return shown("OpenMailFrame"), "open order mail"):
		return false
	await capture("2-order-mail-open")
	if role == "customer-mail":
		var item := OS.get_environment("CRAFTING_OUTPUT_ITEM").to_int()
		var before := bag_count(item)
		await press("OpenMailAttachmentButton1")
		if not await wait_until(func(): return bag_count(item) == before + 1, "receive crafted item"):
			return false
	else:
		var before := money()
		await press("OpenMailMoneyButton")
		if not await wait_until(func(): return money() == before + 10000, "receive commission"):
			return false
	print("CRAFTING MAIL after=", client.mail_state(), " bags=", client.merchant_state().bags)
	await capture("3-order-mail-received")
	return true
