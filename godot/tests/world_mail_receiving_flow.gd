extends "res://tests/world_loot_options_flow.gd"

# Main runs this only AFTER its game-cli AH/mail proof, on an owned loopback server.
# Startup must select the prepared recipient within 5 yd of the actual mailbox.
# Required env: GODOT_TEST_SERVER, GODOT_MAIL_ENTRY, GODOT_MAIL_DISPLAY,
# GODOT_MAIL_MODEL, GODOT_MAIL_MONEY_ID, GODOT_MAIL_ITEM_IDS (won,returned mail ids).
# No direct requests, injected mailbox data, synthetic objects, sending or COD payment.
const MAIL_WAIT_MS := 15000

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var entry := OS.get_environment("GODOT_MAIL_ENTRY").to_int()
	var display := OS.get_environment("GODOT_MAIL_DISPLAY").to_int()
	var model := OS.get_environment("GODOT_MAIL_MODEL").to_int()
	var money_id := OS.get_environment("GODOT_MAIL_MONEY_ID").to_int()
	var item_ids := OS.get_environment("GODOT_MAIL_ITEM_IDS").split(",", false)
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or entry <= 0 or display <= 0 or model <= 0 or money_id <= 0 or item_ids.size() < 2:
		fail("Mail receiving fixture requires owned loopback endpoint, actual mailbox metadata, proceeds and won/returned mail ids")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "InWorld", 180000):
		return
	var object := await find_mailbox(client, entry, display, model)
	if object.is_empty():
		return
	await corpse_click(object.point, false)
	if not await wait_mail_open(client, object.id):
		return
	if not await select_mail(client, money_id):
		return
	var proceeds := inbox_mail(client, money_id)
	if proceeds.is_empty() or proceeds.money <= 0 or proceeds.cod != 0:
		fail("Prepared proceeds mail must contain money and no COD")
		return
	var before_money: int = client.mail_state().money
	var before_bags: Array = client.merchant_state().bags.duplicate(true)
	if not await click_mail(client, "OpenMailMoneyButton"):
		return
	if not await wait_money_claim(client, money_id, before_money + int(proceeds.money)):
		return
	if client.merchant_state().bags != before_bags:
		fail("Money claim changed inventory")
		return
	for raw_id in item_ids:
		var id := raw_id.to_int()
		if id <= 0 or not await select_mail(client, id):
			return
		var mail := inbox_mail(client, id)
		if mail.is_empty() or mail.cod != 0 or mail.attachments.is_empty():
			fail("Prepared won/returned mail must contain non-COD items")
			return
		var attachments: Array = mail.attachments.duplicate(true)
		for attachment in attachments:
			var count := bag_count(client, int(attachment.item_id))
			var money: int = client.mail_state().money
			if not await click_mail(client, "OpenMailAttachmentButton%s" % (int(attachment.slot) + 1)):
				return
			if not await wait_item_claim(client, id, int(attachment.slot), int(attachment.item_id), count + int(attachment.count), money):
				return
	var claimed_money: int = client.mail_state().money
	var claimed_bags: Array = client.merchant_state().bags.duplicate(true)
	if not await click_mail(client, "MailFrameCloseButton"):
		return
	await wait_frames(8)
	if client.mail_state().open or client.get_node_or_null("MailboxUI") != null:
		fail("Mailbox close did not remove receiving UI")
		return
	object = await find_mailbox(client, entry, display, model)
	if object.is_empty():
		return
	await corpse_click(object.point, false)
	if not await wait_mail_open(client, object.id):
		return
	await create_timer(0.25).timeout
	if client.mail_state().money != claimed_money or client.merchant_state().bags != claimed_bags:
		fail("Reopening mailbox replayed a claim")
		return
	if not await click_mail(client, "MailFrameCloseButton"):
		return
	print("PASS: actual mailbox model/pick -> mail role/contents -> authored receiving UI -> proceeds/won/returned claims -> authoritative Gold/inventory -> quiet reopen")
	client.free()
	quit(0)

func find_mailbox(client: Node, entry: int, display: int, fdid: int) -> Dictionary:
	var deadline := Time.get_ticks_msec() + MAIL_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var camera := root.get_camera_3d()
		if camera == null:
			continue
		for object in client.get_children():
			if not object is Node3D or not object.has_meta("game_object_entry") or int(object.get_meta("game_object_entry")) != entry:
				continue
			var model := object.get_node_or_null("GameObjectModel") as Node3D
			if int(object.get_meta("game_object_display_id")) != display or model == null or int(model.get_meta("model_file_data_id")) != fdid:
				fail("Mailbox did not use its actual display/model metadata")
				return {}
			var id: int = object.get_meta("game_object_server_id")
			for candidate in model.find_children("*", "MeshInstance3D", true, false):
				var mesh := candidate as MeshInstance3D
				if mesh == null or mesh.mesh == null or not mesh.is_visible_in_tree():
					continue
				var palette := corpse_bone_palette(mesh)
				for surface in range(mesh.mesh.get_surface_count()):
					for center in corpse_triangle_centroids(mesh, surface, palette):
						var point := camera.unproject_position(center)
						if camera.is_position_in_frustum(center) and Rect2(Vector2.ZERO, Vector2(root.size)).has_point(point) and UnitPicker.pick(camera, point) == id:
							return {"id": id, "point": point}
	fail("Actual replicated mailbox had no visible, pickable model triangle")
	return {}

func wait_mail_open(client: Node, object: int) -> bool:
	var deadline := Time.get_ticks_msec() + MAIL_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.mail_state()
		var host := client.get_node_or_null("MailboxUI")
		if state.open and state.object == object and host != null:
			var frame := host.find_child("MailFrame", true, false) as Control
			var bag := host.find_child("ContainerFrame0", true, false) as Control
			if frame != null and frame.is_visible_in_tree() and bag != null and bag.is_visible_in_tree() and not state.mails.is_empty():
				if host.find_child("SendMailNameEditBox", true, false) != null or host.find_child("MailFrameTab2", true, false) != null:
					fail("Receiving-only slice exposed sending controls")
					return false
				return true
	fail("UseGameObject did not open actual mailbox contents, MailFrame and backpack")
	return false

func inbox_mail(client: Node, id: int) -> Dictionary:
	for mail in client.mail_state().mails:
		if int(mail.id) == id:
			return mail
	return {}

func select_mail(client: Node, id: int) -> bool:
	var deadline := Time.get_ticks_msec() + MAIL_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.mail_state()
		if state.busy:
			continue
		var index := -1
		for i in range(state.mails.size()):
			if int(state.mails[i].id) == id:
				index = i
		if index < 0:
			fail("Prepared AH mail id absent: %s" % id)
			return false
		var page := floori(float(index) / 7.0)
		if state.page != page:
			if not await click_mail(client, "InboxNextPageButton" if state.page < page else "InboxPrevPageButton"):
				return false
			continue
		if not await click_mail(client, "MailItem%sButton" % (index % 7 + 1)):
			return false
		while Time.get_ticks_msec() < deadline:
			await process_frame
			state = client.mail_state()
			if state.selected == id and not state.busy:
				var mail := inbox_mail(client, id)
				var host := client.get_node_or_null("MailboxUI")
				var subject := host.find_child("OpenMailSubject", true, false) as Label if host != null else null
				var sender := host.find_child("OpenMailSenderName", true, false) as Label if host != null else null
				if subject == null or sender == null or not subject.is_visible_in_tree() or subject.text != mail.subject or sender.text.is_empty():
					fail("Authored OpenMailFrame did not render received subject/sender")
					return false
				return true
	fail("Received mail did not open/mark read through native row input")
	return false

func click_mail(client: Node, name: String) -> bool:
	var host := client.get_node_or_null("MailboxUI")
	var control := host.find_child(name, true, false) as Control if host != null else null
	if control == null or not control.is_visible_in_tree():
		fail("Authored receiving control absent: " + name)
		return false
	await click(control)
	return true

func bag_count(client: Node, item: int) -> int:
	var count := 0
	for stack in client.merchant_state().bags:
		if int(stack.item_id) == item:
			count += int(stack.count)
	return count

func wait_money_claim(client: Node, id: int, expected: int) -> bool:
	var deadline := Time.get_ticks_msec() + MAIL_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var mail := inbox_mail(client, id)
		if not client.mail_state().busy and client.mail_state().money == expected and (mail.is_empty() or mail.money == 0):
			return true
	fail("Proceeds claim did not update authoritative Gold and mailbox")
	return false

func wait_item_claim(client: Node, id: int, slot: int, item: int, count: int, money: int) -> bool:
	var deadline := Time.get_ticks_msec() + MAIL_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var mail := inbox_mail(client, id)
		var remains := false
		for attachment in mail.get("attachments", []):
			remains = remains or int(attachment.slot) == slot
		if not remains and not client.mail_state().busy and bag_count(client, item) == count and client.mail_state().money == money:
			return true
	fail("Item claim did not remove the exact attachment, update inventory count, and preserve Gold")
	return false
