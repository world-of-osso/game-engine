extends SceneTree

# Observe only. Production root startup + actual JS supply every input/action.
# Markers release protocol replies; no direct text/input/pressed/account/dump calls.
const CHARACTER := "JS World Fixture"
const VENDOR := "Fixture Vendor"
const FIRST := Vector3(-8949.0, 112.879913, 0.0)
const QUIET_MS := 900
var client: Node
var artifacts: String
var saw_credentials := false
var saw_connect := false
var saw_enter := false
var npc_id: Variant
var vendor_texture: Texture2D
var stage := "setup"

func _initialize() -> void:
	call_deferred("run")

func observe_node(node: Node) -> void:
	if node.name == "UsernameInput" and node is LineEdit:
		node.text_changed.connect(observe_credentials)
	elif node.name == "PasswordInput" and node is LineEdit:
		node.text_changed.connect(observe_credentials)
	elif node.name == "ConnectButton" and node is Button:
		node.pressed.connect(func():
			observe_credentials()
			saw_connect = saw_credentials)
	elif node.name == "EnterWorld" and node is Button:
		node.pressed.connect(func(): saw_enter = true)

func observe_credentials(_text: String = "") -> void:
	var username := client.find_child("UsernameInput", true, false) as LineEdit
	var password := client.find_child("PasswordInput", true, false) as LineEdit
	if username != null and password != null:
		saw_credentials = saw_credentials or (username.text == OS.get_environment("LOGIN_USER") and password.text == OS.get_environment("LOGIN_PASS") and password.secret)

func mark(name: String) -> bool:
	var file := FileAccess.open(artifacts.path_join(name), FileAccess.WRITE)
	if file == null:
		fail("SETUP: marker write failed: " + name)
		return false
	file.store_string("observed\n")
	file.close()
	return true

func fail(message: String) -> void:
	# Do not print account feedback or credentials. Never label setup failure JS RED.
	push_error("JS WORLD " + stage + ": " + message)
	if is_instance_valid(client):
		client.queue_free()
	quit(1)

func snapshot(name: String) -> bool:
	var account: Dictionary = client.account_state()
	var file := FileAccess.open(artifacts.path_join("observation.jsonl"), FileAccess.READ_WRITE)
	if file == null:
		file = FileAccess.open(artifacts.path_join("observation.jsonl"), FileAccess.WRITE)
	if file == null:
		fail("SETUP: authority evidence file unavailable")
		return false
	file.seek_end()
	# Root GameClient views of replicated money and server-owned inventory, not fixture counters.
	file.store_line(JSON.stringify({"stage": name, "ticks_ms": Time.get_ticks_msec(), "screen": account.screen, "selected_id": account.selected_character_id, "selected_name": account.selected_character_name, "local_player_id": account.local_player_id, "local_server_position": account.local_server_position, "unit_count": account.unit_count, "merchant": client.merchant_state()}))
	file.close()
	print("OBSERVE JS WORLD ", name)
	return true

func wait_until(predicate: Callable, timeout_ms: int, description: String) -> bool:
	stage = description
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("deadline; inspect native.log; missing setup/readiness is not automatic feature RED")
	return false

func control(name: String) -> Control:
	var ui := client.get_node_or_null("MerchantUI")
	return ui.find_child(name, true, false) as Control if ui != null else null

func texture(node: Control) -> TextureRect:
	if node == null:
		return null
	if node is TextureRect and node.is_visible_in_tree() and node.texture != null:
		return node
	for child in node.find_children("*", "TextureRect", true, false):
		if child.is_visible_in_tree() and child.texture != null:
			return child
	return null

func charselect_account_ready(state: Dictionary) -> bool:
	var login_completed := saw_credentials and saw_connect and state.reply_received
	if not login_completed:
		return false
	var roster_matches := state.screen == "CharacterSelect" and state.character_count == 1
	return roster_matches

func charselect_presentation_ready(selection: CanvasLayer, selected: Label, enter: Button) -> bool:
	if selection == null or not selection.visible:
		return false
	if selected == null or selected.text != CHARACTER:
		return false
	return enter != null and enter.is_visible_in_tree()

func charselect_ready() -> bool:
	var state: Dictionary = client.account_state()
	var selection := client.get_node_or_null("CharacterSelectUI") as CanvasLayer
	var selected := client.find_child("CharSelectCharacterName", true, false) as Label
	var enter := client.find_child("EnterWorld", true, false) as Button
	return charselect_account_ready(state) and charselect_presentation_ready(selection, selected, enter)

func loading_ready() -> bool:
	var loading := client.get_node_or_null("LoadingUI") as CanvasLayer
	return saw_enter and client.account_state().screen == "Loading" and loading != null and loading.visible and client.get_node_or_null("CharacterSelectScene") == null

func world_account_ready(state: Dictionary) -> bool:
	if state.screen != "InWorld" or not state.reply_received:
		return false
	if state.selected_character_id != 17 or state.selected_character_name != CHARACTER:
		return false
	return state.unit_count == 1 and state.world_attached and state.gameplay_input_allowed

func terrain_ready(terrain: Dictionary) -> bool:
	var map_idle := terrain.map == "azeroth" and terrain.pending_count == 0
	if not map_idle:
		return false
	var tiles_ready := terrain.failures.is_empty() and not terrain.parsed_tiles.is_empty()
	if not tiles_ready:
		return false
	for tile in terrain.parsed_tiles:
		if tile.chunk_count <= 0 or not FileAccess.file_exists(tile.root_path):
			return false
	return true

func world_presentation_ready(loading: CanvasLayer, terrain_root: Node, player: Node3D) -> bool:
	if loading == null or loading.visible:
		return false
	if terrain_root == null or terrain_root.get_child_count() == 0:
		return false
	if player == null or player.position.distance_to(FIRST) > 0.5:
		return false
	return true

func player_grounded(player: Node3D) -> bool:
	var height: Variant = client.terrain_height_at(player.position.x, player.position.z)
	if height == null or absf(player.position.y - float(height)) >= 0.3:
		return false
	return true

func player_visual_ready(player: Node3D) -> bool:
	# Existing helpers, read-only subset: no paused clocks/pose setters/capture mutation.
	var equipment = load("res://tests/world_player_equipment_pixels.gd").new()
	var visual: Node3D = equipment.find_visual(player)
	if visual == null or equipment.inspect_visual(visual, true) != "":
		return false
	var locomotion = load("res://tests/player_locomotion_probe.gd").new()
	if locomotion.bind(player) != "" or locomotion.animation.current_animation_id() != 0:
		return false
	var poses: Array[Transform3D] = locomotion.capture_pose()
	return not poses.is_empty() and poses.all(func(pose): return pose.origin.is_finite() and pose.basis.is_finite())

func world_ready() -> bool:
	var state: Dictionary = client.account_state()
	if not world_account_ready(state):
		return false
	var terrain: Dictionary = state.terrain
	if not terrain_ready(terrain):
		return false
	var loading := client.get_node_or_null("LoadingUI") as CanvasLayer
	var terrain_root := client.get_node_or_null("WorldTerrain")
	var player := client.get_node_or_null("WorldUnits/" + CHARACTER) as Node3D
	if not world_presentation_ready(loading, terrain_root, player):
		return false
	return player_grounded(player) and player_visual_ready(player)

func npc_ready() -> bool:
	var vendor := client.get_node_or_null("WorldUnits/" + VENDOR) as Node3D
	if vendor == null or client.account_state().unit_count != 2:
		return false
	var equipment = load("res://tests/world_player_equipment_pixels.gd").new()
	return vendor.position.distance_to(FIRST + Vector3(-2.0, 0.1, 3.0)) <= 0.5 and equipment.has_visible_mesh(vendor)

func visible_label_matches(label: Label, text: String) -> bool:
	return label != null and label.is_visible_in_tree() and label.text == text

func merchant_source_ready(source: Control) -> bool:
	return source != null and source.is_visible_in_tree() and source.get_global_rect().has_area()

func merchant_item_content_matches(name_label: Label, icon: TextureRect, count: Label, price: Label) -> bool:
	if not visible_label_matches(name_label, "Linen Cloth"):
		return false
	if icon == null or icon.texture != vendor_texture:
		return false
	var count_hidden := count == null or not count.is_visible_in_tree()
	return count_hidden and visible_label_matches(price, "25")

func source_matches() -> bool:
	var name_label := control("MerchantItem1Name") as Label
	var count := control("MerchantItem1ItemButtonCount") as Label
	var price := control("MerchantItem1MoneyFrameAmount0") as Label
	var icon := texture(control("MerchantItem1ItemButtonIcon"))
	var source := control("MerchantItem1")
	return merchant_source_ready(source) and merchant_item_content_matches(name_label, icon, count, price)

func picker_matches(amount: String) -> bool:
	var picker := control("StackSplitFrame")
	var shown := picker != null and picker.is_visible_in_tree()
	if shown != (not amount.is_empty()):
		return false
	# Only actual MerchantUI-owned split frame; cursor-owned duplicate must stay hidden.
	for node in client.find_children("StackSplitFrame", "Control", true, false):
		if node != picker and node.is_visible_in_tree():
			return false
	if not shown:
		return true
	var label := control("StackSplitText") as Label
	var canvas := control("RegistryCanvas")
	var source := control("MerchantItem1")
	if label == null or not label.is_visible_in_tree() or label.text != amount or canvas == null or source == null:
		return false
	# Original vendor BOTTOMLEFT = MerchantItem1 TOPLEFT, 172x96 logical pixels.
	var scale := canvas.get_global_transform().get_scale()
	if scale.x <= 0 or scale.y <= 0:
		return false
	var size := Vector2(172.0, 96.0) * scale
	var position := source.get_global_rect().position - Vector2(0.0, size.y)
	return picker.get_global_rect().position.distance_to(position) <= 1.0 and picker.get_global_rect().size.distance_to(size) <= 1.0

func bag_matches(count: int) -> bool:
	var bag := control("ContainerFrame0")
	if bag == null or not bag.is_visible_in_tree():
		return false
	for slot in range(16):
		var prefix := "ContainerFrame0Slot%s" % slot
		var cell := control(prefix)
		var icon := texture(control(prefix + "Icon"))
		var label := control(prefix + "Count") as Label
		if cell == null or not cell.is_visible_in_tree() or not cell.get_global_rect().has_area():
			return false
		var occupied := slot == 0 and count > 0
		if (icon != null) != occupied:
			return false
		# Shared original bag component hides singleton count; stack3 shows exact3.
		if count > 1 and occupied:
			if label == null or not label.is_visible_in_tree() or label.text != str(count):
				return false
		elif label != null and label.is_visible_in_tree():
			return false
	return true

func money_matches(money: int) -> bool:
	# Original coins(): every nonzero denomination, or0 copper. Gold1000 is silver10 only.
	var amounts: Array[String] = []
	for divisor in [10000, 100, 1]:
		var amount: int = (money / divisor) % 100 if divisor != 10000 else money / divisor
		if amount > 0:
			amounts.append(str(amount))
	if amounts.is_empty():
		amounts.append("0")
	for index in range(3):
		var label := control("MerchantMoneyFrameAmount%s" % index) as Label
		if index < amounts.size():
			if label == null or not label.is_visible_in_tree() or label.text != amounts[index]:
				return false
		elif label != null and label.is_visible_in_tree():
			return false
	return true

func merchant_account_matches(account: Dictionary) -> bool:
	return account.screen == "InWorld" and account.selected_character_id == 17

func merchant_session_matches(state: Dictionary) -> bool:
	var session_open := state.open and state.npc == npc_id
	if not session_open:
		return false
	var vendor_matches := state.vendor_name == VENDOR and state.items == ["Linen Cloth"]
	return vendor_matches

func inventory_item_matches(item: Dictionary, count: int) -> bool:
	var slot_matches := item.bag == 0 and item.slot == 0
	if not slot_matches:
		return false
	var content_matches := item.item_id == 2589 and item.name == "Linen Cloth" and item.count == count
	return content_matches

func inventory_authority_matches(state: Dictionary, count: int) -> bool:
	# Visual starter gear is EquipmentAppearance, not an invented occupied inventory slot.
	if not state.equipment.is_empty() or state.bags.size() != (0 if count == 0 else 1):
		return false
	if count > 0:
		var item: Dictionary = state.bags[0]
		if not inventory_item_matches(item, count):
			return false
	return true

func cursor_and_popup_absent() -> bool:
	for name in ["CursorItemIcon", "StaticPopup1"]:
		var node := client.find_child(name, true, false) as Control
		if node != null and node.is_visible_in_tree():
			return false
	return true

func merchant_presentation_matches(count: int, money: int, amount: String) -> bool:
	var source_and_bag_match := client.get_node_or_null("GameMenuUI") == null and source_matches() and bag_matches(count)
	if not source_and_bag_match:
		return false
	var money_and_picker_match := money_matches(money) and picker_matches(amount)
	return money_and_picker_match

func authority_matches(count: int, money: int, amount: String = "") -> bool:
	var account: Dictionary = client.account_state()
	var state: Dictionary = client.merchant_state()
	if not merchant_account_matches(account):
		return false
	if not merchant_session_matches(state):
		return false
	if state.money != money or state.split_open != (not amount.is_empty()):
		return false
	if not inventory_authority_matches(state, count):
		return false
	if not cursor_and_popup_absent():
		return false
	return merchant_presentation_matches(count, money, amount)

func quiet(count: int, money: int, amount: String = "") -> bool:
	var deadline := Time.get_ticks_msec() + QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not authority_matches(count, money, amount):
			fail("authority/presentation changed during900ms; no optimistic bag/Gold allowed")
			return false
	return true

func observe_buy(index: int, before_count: int, before_money: int, after_count: int, after_money: int) -> bool:
	if not await wait_until(func(): return FileAccess.file_exists(artifacts.path_join("decoded%s" % index)), 10000, "decoded%s" % index):
		return false
	# No latency masking: first sample after decoded Buy must still be old authority.
	if not authority_matches(before_count, before_money) or not await quiet(before_count, before_money):
		fail("decoded Buy changed inventory/money before authoritative reply")
		return false
	if not snapshot("commit%s" % index) or not mark("commit%s" % index):
		return false
	if not await wait_until(func(): return authority_matches(after_count, after_money), 6000, "authority%s" % index):
		return false
	if not await quiet(after_count, after_money) or not snapshot("applied%s" % index):
		return false
	return mark("applied%s" % index)

func mount_world_client() -> bool:
	artifacts = OS.get_environment("NATIVE_JS_WORLD_ARTIFACTS")
	var artifacts_available := not artifacts.is_empty()
	var client_class_available := artifacts_available and ClassDB.class_exists("GameClient")
	var native_classes_available := client_class_available and ClassDB.class_exists("WowAnimationPlayer")
	if not native_classes_available:
		fail("SETUP: owned artifacts and current native classes required")
		return false
	root.size = Vector2i(1920, 1080)
	var scene: PackedScene = load("res://scenes/client.tscn")
	if scene == null:
		fail("SETUP: production client scene missing")
		return false
	client = scene.instantiate()
	node_added.connect(observe_node)
	root.add_child(client)
	return true

func observe_world_entry() -> bool:
	if not await wait_until(charselect_ready, 150000, "authored-charselect"):
		return false
	if not mark("charselect"):
		return false
	if not await wait_until(loading_ready, 30000, "actual-loading"):
		return false
	if not mark("loading"):
		return false
	if not await wait_until(world_ready, 180000, "actual-world"):
		return false
	if not snapshot("world-ready"):
		return false
	if not mark("world-ready"):
		return false
	return true

func observe_vendor_baseline() -> bool:
	if not await wait_until(npc_ready, 30000, "replicated-npc"):
		return false
	if not mark("npc-ready"):
		return false
	if not await wait_until(func(): return client.merchant_state().open and texture(control("MerchantItem1ItemButtonIcon")) != null, 6000, "merchant-mount"):
		return false
	npc_id = client.merchant_state().npc
	vendor_texture = texture(control("MerchantItem1ItemButtonIcon")).texture
	if not authority_matches(0, 1000):
		fail("actual vendor baseline/empty inventory/Gold1000 not stable")
		return false
	if not await quiet(0, 1000):
		fail("actual vendor baseline/empty inventory/Gold1000 not stable")
		return false
	if not snapshot("merchant-ready"):
		fail("actual vendor baseline/empty inventory/Gold1000 not stable")
		return false
	if not mark("merchant-ready"):
		fail("actual vendor baseline/empty inventory/Gold1000 not stable")
		return false
	return true

func observe_picker_edits() -> bool:
	if not await wait_until(func(): return authority_matches(1, 975, "1"), 10000, "actual-picker1"):
		return false
	if not snapshot("picker1"):
		return false
	if not mark("picker1"):
		return false
	if not await wait_until(func(): return authority_matches(1, 975, "2"), 6000, "actual-key2-picker"):
		return false
	if not snapshot("picker2"):
		return false
	if not mark("picker2"):
		return false
	return true

func observe_final_authority() -> void:
	# Script has5s post-Enter wait then its own live dump. Observer never calls dumps.
	if not await quiet(3, 925):
		return
	if not snapshot("done"):
		return
	if not mark("done"):
		return
	var deadline := Time.get_ticks_msec() + 4000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not authority_matches(3, 925):
			fail("final authority changed while JS finishes")
			return
	client.queue_free()
	await process_frame
	quit(0)

func run() -> void:
	if not mount_world_client():
		return
	if not await observe_world_entry():
		return
	if not await observe_vendor_baseline():
		return
	if not await observe_buy(1, 0, 1000, 1, 975):
		return
	if not await observe_picker_edits():
		return
	if not await observe_buy(2, 1, 975, 3, 925):
		return
	await observe_final_authority()
