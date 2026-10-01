extends "res://tests/world_mail_receiving_flow.gd"

# Two-client player mail on an owned loopback server. One process per role, started with
# the prepared character beside a real mailbox (--server, --screen inworld, --char after
# Godot's `--`). Roles meet through flag files the orchestrator also writes; it delivers
# item mail in transit with `game-server-admin mail-deliver-now` and restarts the server.
# Every request comes from real frame clicks, right-clicks and typed keys.
# Env: GODOT_TEST_SERVER, GODOT_MAIL_ROLE (sender | recipient | sender_collect |
# verify_recipient | verify_sender), GODOT_MAIL_SYNC, GODOT_MAIL_SCREENSHOTS,
# GODOT_MAIL_ENTRY/DISPLAY/MODEL, GODOT_MAIL_SENDER, GODOT_MAIL_RECIPIENT.
const FLAG_WAIT_MS := 600000
const STEP_MS := 15000
const LINEN := 2589
const SILK := 4306
const WOOL := 2592
const POSTAGE := 30
const MONEY_SENT := 50000
const COD := 10000

var sync_dir := ""
var shots := ""
var sender_name := ""
var recipient_name := ""

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	sync_dir = OS.get_environment("GODOT_MAIL_SYNC")
	shots = OS.get_environment("GODOT_MAIL_SCREENSHOTS")
	sender_name = OS.get_environment("GODOT_MAIL_SENDER")
	recipient_name = OS.get_environment("GODOT_MAIL_RECIPIENT")
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var role := OS.get_environment("GODOT_MAIL_ROLE")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or sync_dir.is_empty() or shots.is_empty() or sender_name.is_empty() or recipient_name.is_empty():
		fail("Mail flow requires an owned loopback endpoint, sync and screenshot directories and both names")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "InWorld", 180000):
		return
	await create_timer(3.0).timeout
	var ok := false
	match role:
		"sender": ok = await run_sender(client)
		"recipient": ok = await run_recipient(client)
		"recipient_return": ok = await run_recipient_return(client)
		"sender_collect": ok = await run_sender_collect(client)
		"verify_recipient": ok = await run_verify_recipient(client)
		"verify_sender": ok = await run_verify_sender(client)
		_: fail("Unknown GODOT_MAIL_ROLE: " + role)
	if not ok:
		return
	print("PASS: mail role ", role)
	client.free()
	quit(0)

# --- Roles ---

func run_sender(client: Node) -> bool:
	if not await open_mailbox(client):
		return false
	if not await click_mail(client, "MailFrameTab2") or not await wait_state(client, func(s): return s.tab == "send", "Send Mail tab"):
		return false
	await shot("a01-send-tab.png")
	# Refusals keep the form and the money.
	for refused in [["Nobodyhere", "Cannot find mail recipient."], [sender_name, "You can't send mail to yourself."]]:
		var gold: int = client.mail_state().money
		if not await type_text(client, "SendMailNameEditBox", refused[0]) or not await type_text(client, "SendMailSubjectEditBox", "Hello"):
			return false
		if not await click_mail(client, "SendMailMailButton") or not await wait_error(client, refused[1]):
			return false
		await shot("a02-refused-%s.png" % refused[0].to_lower())
		if client.mail_state().money != gold or client.mail_state().texts.SendMailSubjectEditBox != "Hello":
			fail("Refused send changed money or cleared the form")
			return false
		if not await click_mail(client, "SendMailCancelButton"):
			return false
	# 1. Money.
	if not await fill_form(client, recipient_name, "Gold for you", "Five gold, as promised.") or not await type_text(client, "SendMailMoneyGold", "5"):
		return false
	await shot("a03-money-form.png")
	if not await send_and_wait(client, MONEY_SENT + POSTAGE, -1, 0):
		return false
	await shot("a04-money-sent.png")
	# 2. An item; the subject fills in from it.
	if not await attach(client, LINEN) or not await wait_text(client, "SendMailSubjectEditBox", "Linen Cloth (20)"):
		return false
	if not await fill_form(client, recipient_name, "", "Linen for your tailoring."):
		return false
	await shot("a05-item-form.png")
	if not await send_and_wait(client, POSTAGE, LINEN, 20):
		return false
	# 3. C.O.D.
	if not await attach(client, SILK) or not await click_mail(client, "SendMailCODButton") or not await wait_state(client, func(s): return s.cod_mode, "C.O.D. mode"):
		return false
	if not await fill_form(client, recipient_name, "", "Pay on delivery.") or not await type_text(client, "SendMailMoneyGold", "1"):
		return false
	await shot("a06-cod-form.png")
	if not await send_and_wait(client, POSTAGE, SILK, 10):
		return false
	# 4. An item the recipient returns, and 5. a letter left unread across the restart.
	if not await attach(client, WOOL) or not await fill_form(client, recipient_name, "Please return", "Send this back."):
		return false
	if not await send_and_wait(client, POSTAGE, WOOL, 5):
		return false
	if not await fill_form(client, recipient_name, "Read after restart", "This letter outlives a server restart."):
		return false
	if not await send_and_wait(client, POSTAGE, -1, 0):
		return false
	await shot("a07-all-sent.png")
	write_flag("A1", str(client.mail_state().money))
	return true

func run_recipient(client: Node) -> bool:
	if not await wait_flag("A1"):
		return false
	if not await wait_minimap_mail(client, [sender_name]):
		return false
	await shot("b01-minimap-new-mail.png")
	if not await hover_minimap_mail(client, [sender_name]):
		return false
	await shot("b02-minimap-mail-tooltip.png")
	if not await open_mailbox(client):
		return false
	# Money and letters arrive at once; items to another account's character wait an hour.
	if sorted(subjects(client)) != ["Gold for you", "Read after restart"]:
		fail("Expected only instant mail before delivery: " + str(subjects(client)))
		return false
	await shot("b03-inbox-before-item-delivery.png")
	write_flag("B1", "")
	if not await wait_flag("D1") or not await wait_state(client, func(s): return s.mails.size() == 5, "delivered item mail"):
		return false
	await shot("b04-inbox-delivered.png")
	# Money, then delete the emptied letter.
	var money_mail := mail_by_subject(client, "Gold for you")
	var gold: int = client.mail_state().money
	if not await select_mail(client, int(money_mail.id)) or not await click_mail(client, "OpenMailMoneyButton"):
		return false
	if not await wait_state(client, func(s): return s.money == gold + MONEY_SENT and not s.busy, "money taken"):
		return false
	await shot("b05-money-taken.png")
	if not await click_mail(client, "OpenMailDeleteButton") or not await wait_state(client, func(s): return mail_by_subject_in(s, "Gold for you").is_empty(), "letter deleted"):
		return false
	# The item.
	var item_mail := mail_by_subject(client, "Linen Cloth (20)")
	if not await take_first(client, item_mail, LINEN, 20):
		return false
	await shot("b06-item-taken.png")
	# C.O.D.: confirm, pay, take.
	var cod_mail := mail_by_subject(client, "Silk Cloth (10)")
	if int(cod_mail.get("cod", 0)) != COD:
		fail("C.O.D. mail missing or wrong amount: " + str(cod_mail))
		return false
	gold = client.mail_state().money
	var silk := bag_count(client, SILK)
	if not await select_mail(client, int(cod_mail.id)) or not await click_mail(client, "OpenMailAttachmentButton1"):
		return false
	if not await wait_popup(client, "Accepting this item will cost:\n1g"):
		return false
	await shot("b07-cod-confirmation.png")
	if not await click_popup(client, "StaticPopup1Button1"):
		return false
	if not await wait_state(client, func(s): return s.money == gold - COD and not s.busy, "C.O.D. paid") or not await wait_bag(client, SILK, silk + 10):
		return false
	await shot("b08-cod-paid.png")
	return await return_and_close(client)

# The returned mail is the last recipient step; `recipient_return` resumes a run here.
func run_recipient_return(client: Node) -> bool:
	return await open_mailbox(client) and await return_and_close(client)

func return_and_close(client: Node) -> bool:
	var back := mail_by_subject(client, "Please return")
	if not await select_mail(client, int(back.id)) or not label_is(client, "OpenMailDeleteButton", "Return"):
		return false
	await shot("b09-return-button.png")
	if not await click_mail(client, "OpenMailDeleteButton") or not await wait_state(client, func(s): return mail_by_subject_in(s, "Please return").is_empty(), "mail returned"):
		return false
	await shot("b10-after-return.png")
	if not await click_mail(client, "MailFrameCloseButton"):
		return false
	write_flag("B2", str(client.mail_state().money))
	return true

func run_sender_collect(client: Node) -> bool:
	if not await wait_flag("B2"):
		return false
	if not await wait_minimap_mail(client, [recipient_name]):
		return false
	await shot("a08-minimap-cod-payment.png")
	if not await wait_flag("D2") or not await open_mailbox(client):
		return false
	if not await wait_state(client, func(s): return s.mails.size() == 2, "C.O.D. payment and returned mail"):
		return false
	await shot("a09-inbox-payment-and-return.png")
	var payment := mail_by_subject(client, "Silk Cloth (10)")
	if int(payment.money) != COD or payment.sender != recipient_name or payment.can_delete != true:
		fail("C.O.D. payment mail wrong: " + str(payment))
		return false
	var gold: int = client.mail_state().money
	if not await select_mail(client, int(payment.id)) or not await click_mail(client, "OpenMailMoneyButton"):
		return false
	if not await wait_state(client, func(s): return s.money == gold + COD and not s.busy, "C.O.D. payment taken"):
		return false
	await shot("a10-cod-payment-taken.png")
	var returned := mail_by_subject(client, "Please return")
	if not returned.returned:
		fail("Returned mail not marked returned: " + str(returned))
		return false
	if not await take_first(client, returned, WOOL, 5):
		return false
	await shot("a11-returned-item-taken.png")
	write_flag("A2", str(client.mail_state().money))
	return true

func run_verify_recipient(client: Node) -> bool:
	if bag_count(client, LINEN) != 20 or bag_count(client, SILK) != 10:
		fail("Taken items did not persist: " + str(client.merchant_state().bags))
		return false
	if not await wait_minimap_mail(client, [sender_name]):
		return false
	await shot("c01-restart-minimap.png")
	if not await open_mailbox(client):
		return false
	var letter := mail_by_subject(client, "Read after restart")
	if sorted(subjects(client)) != ["Linen Cloth (20)", "Read after restart", "Silk Cloth (10)"] or letter.read:
		fail("Mailbox after restart: " + str(client.mail_state().mails))
		return false
	if not await select_mail(client, int(letter.id)):
		return false
	await shot("c02-restart-letter.png")
	if not await click_mail(client, "OpenMailDeleteButton") or not await wait_state(client, func(s): return mail_by_subject_in(s, "Read after restart").is_empty(), "letter deleted"):
		return false
	write_flag("C1", str(client.mail_state().money))
	return true

func run_verify_sender(client: Node) -> bool:
	if bag_count(client, WOOL) != 5 or bag_count(client, LINEN) != 0 or bag_count(client, SILK) != 0:
		fail("Sender bags after restart: " + str(client.merchant_state().bags))
		return false
	if not await open_mailbox(client):
		return false
	await shot("c03-restart-sender-mailbox.png")
	write_flag("C2", str(client.mail_state().money))
	return true

# --- Steps ---

func open_mailbox(client: Node) -> bool:
	var entry := OS.get_environment("GODOT_MAIL_ENTRY").to_int()
	var object := await find_mailbox(client, entry, OS.get_environment("GODOT_MAIL_DISPLAY").to_int(), OS.get_environment("GODOT_MAIL_MODEL").to_int())
	if object.is_empty():
		return false
	await corpse_click(object.point, false)
	if not await wait_state(client, func(s): return s.open and s.object == object.id and s.has("now") and not s.busy, "mailbox open"):
		return false
	var host := client.get_node_or_null("MailboxUI")
	var frame := host.find_child("MailFrame", true, false) as Control if host != null else null
	if frame == null or not frame.is_visible_in_tree():
		fail("MailFrame not shown")
		return false
	await create_timer(0.5).timeout
	return true

func fill_form(client: Node, to: String, subject: String, body: String) -> bool:
	if not await type_text(client, "SendMailNameEditBox", to):
		return false
	if not subject.is_empty() and not await type_text(client, "SendMailSubjectEditBox", subject):
		return false
	return await type_text(client, "SendMailBodyEditBox", body)

# Click the edit box, then type its text one key at a time.
func type_text(client: Node, name: String, text: String) -> bool:
	var host := client.get_node_or_null("MailboxUI")
	var edit := host.find_child(name, true, false) as LineEdit if host != null else null
	if edit == null or not edit.is_visible_in_tree():
		fail("Edit box absent: " + name)
		return false
	await click(edit)
	if not edit.has_focus():
		fail("Edit box did not take focus: " + name)
		return false
	if not edit.text.is_empty():
		for code in [KEY_A, KEY_BACKSPACE]:
			var erase := InputEventKey.new()
			erase.keycode = code
			erase.ctrl_pressed = code == KEY_A
			erase.pressed = true
			root.push_input(erase, true)
			await process_frame
	for character in text:
		var key := InputEventKey.new()
		key.keycode = OS.find_keycode_from_string(character.to_upper()) if character != " " else KEY_SPACE
		key.unicode = character.unicode_at(0)
		key.pressed = true
		root.push_input(key, true)
		await process_frame
		key = key.duplicate()
		key.pressed = false
		root.push_input(key, true)
	await process_frame
	return await wait_text(client, name, text)

func wait_text(client: Node, name: String, text: String) -> bool:
	return await wait_state(client, func(s): return s.texts.get(name, "") == text, "%s = %s" % [name, text])

func attach(client: Node, item_id: int) -> bool:
	var stack := bag_stack(client, item_id)
	if stack.is_empty():
		fail("No bag stack of %s" % item_id)
		return false
	var host := client.get_node_or_null("MailboxUI")
	var slot := host.find_child("ContainerFrame%sSlot%s" % [stack.bag, stack.slot], true, false) as Control if host != null else null
	if slot == null or not slot.is_visible_in_tree():
		fail("Bag slot control absent for %s" % item_id)
		return false
	var count: int = client.mail_state().attachments.size()
	await right_click(slot)
	return await wait_state(client, func(s): return s.attachments.size() == count + 1 and int(s.attachments[-1].item_id) == item_id, "attached %s" % item_id)

# Send and wait for MailSent: the form clears and the server takes postage (+money).
func send_and_wait(client: Node, cost: int, item_id: int, count: int) -> bool:
	var gold: int = client.mail_state().money
	var before := bag_count(client, item_id) if item_id > 0 else 0
	if not await click_mail(client, "SendMailMailButton"):
		return false
	if not await wait_state(client, func(s): return not s.busy and s.attachments.is_empty() and s.texts.get("SendMailNameEditBox", "x") == "" and s.money == gold - cost, "mail sent"):
		return false
	if not await wait_error(client, "Mail sent."):
		return false
	return item_id <= 0 or await wait_bag(client, item_id, before - count)

func take_first(client: Node, mail: Dictionary, item_id: int, count: int) -> bool:
	if mail.is_empty() or mail.attachments.is_empty():
		fail("Mail without attachments: " + str(mail))
		return false
	var before := bag_count(client, item_id)
	var slot := int(mail.attachments[0].slot)
	if not await select_mail(client, int(mail.id)) or not await click_mail(client, "OpenMailAttachmentButton%s" % (slot + 1)):
		return false
	return await wait_bag(client, item_id, before + count)

func wait_minimap_mail(client: Node, senders: Array) -> bool:
	var deadline := Time.get_ticks_msec() + STEP_MS * 4
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var ui := client.get_node_or_null("MinimapUI")
		var icon := ui.find_child("MiniMapMailFrame", true, false) as Control if ui != null else null
		if icon != null and icon.is_visible_in_tree() and client.mail_state().pending_senders == senders:
			return true
	fail("Minimap mail icon did not show for %s: %s" % [senders, client.mail_state().pending_senders])
	return false

func hover_minimap_mail(client: Node, senders: Array) -> bool:
	var icon := client.get_node("MinimapUI").find_child("MiniMapMailFrame", true, false) as Control
	var motion := InputEventMouseMotion.new()
	motion.position = icon.get_global_rect().get_center()
	root.push_input(motion, true)
	var deadline := Time.get_ticks_msec() + STEP_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var tooltip := client.get_node_or_null("BagTooltipUI")
		if tooltip == null:
			continue
		var texts: Array = tooltip.find_children("*", "Label", true, false).filter(func(l): return l.is_visible_in_tree()).map(func(l): return l.text)
		if texts.has("Unread mail from:") and senders.all(func(s): return texts.has(s)):
			return true
	fail("Minimap mail tooltip did not name " + str(senders))
	return false

func wait_popup(client: Node, text: String) -> bool:
	var deadline := Time.get_ticks_msec() + STEP_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var ui := client.get_node_or_null("StaticPopupUI")
		if ui == null:
			continue
		for node in ui.find_children("*", "Label", true, false):
			if node.is_visible_in_tree() and node.text == text:
				return true
	fail("Popup not shown: " + text)
	return false

func click_popup(client: Node, name: String) -> bool:
	var ui := client.get_node_or_null("StaticPopupUI")
	var button := ui.find_child(name, true, false) as Control if ui != null else null
	if button == null or not button.is_visible_in_tree():
		fail("Popup button absent: " + name)
		return false
	await click(button)
	return true

func wait_error(client: Node, text: String) -> bool:
	var deadline := Time.get_ticks_msec() + STEP_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var errors := client.get_node_or_null("UIErrors")
		if errors == null:
			continue
		for node in errors.find_children("*", "Label", true, false):
			if node.is_visible_in_tree() and node.text == text:
				return true
	fail("UI error not shown: " + text)
	return false

func wait_bag(client: Node, item_id: int, count: int) -> bool:
	var deadline := Time.get_ticks_msec() + STEP_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if bag_count(client, item_id) == count:
			return true
	fail("Bag count of %s is %s, expected %s" % [item_id, bag_count(client, item_id), count])
	return false

func wait_state(client: Node, check: Callable, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + STEP_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if check.call(client.mail_state()):
			return true
	fail("Timed out waiting for " + what + ": " + str(client.mail_state()))
	return false

func label_is(client: Node, name: String, text: String) -> bool:
	var host := client.get_node_or_null("MailboxUI")
	var control := host.find_child(name, true, false) if host != null else null
	var label := control.find_child("Text", true, false) as Label if control != null else null
	var shown: String = label.text if label != null else ""
	if shown != text:
		fail("%s does not read %s" % [name, text])
		return false
	return true

func right_click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_RIGHT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func bag_stack(client: Node, item_id: int) -> Dictionary:
	for stack in client.merchant_state().bags:
		if int(stack.item_id) == item_id:
			return stack
	return {}

func subjects(client: Node) -> Array:
	return client.mail_state().mails.map(func(m): return m.subject)

func sorted(values: Array) -> Array:
	var copy := values.duplicate()
	copy.sort()
	return copy

func mail_by_subject(client: Node, subject: String) -> Dictionary:
	return mail_by_subject_in(client.mail_state(), subject)

func mail_by_subject_in(state: Dictionary, subject: String) -> Dictionary:
	for mail in state.mails:
		if mail.subject == subject:
			return mail
	return {}

func write_flag(name: String, content: String) -> void:
	var file := FileAccess.open(sync_dir.path_join(name), FileAccess.WRITE)
	file.store_string(content)
	file.close()

func wait_flag(name: String) -> bool:
	var deadline := Time.get_ticks_msec() + FLAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if FileAccess.file_exists(sync_dir.path_join(name)):
			return true
	fail("Flag never written: " + name)
	return false

func shot(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.is_empty() or image.save_png(shots.path_join(file)) != OK:
		push_error("Screenshot failed: " + file)
