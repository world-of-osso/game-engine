extends "res://tests/world_mail_flow.gd"

# Prepared private recipient beside a real mailbox, with the authentic Outbid mail.
# Required: GODOT_TEST_SERVER, GODOT_TEST_ACCOUNT, GODOT_MAIL_ENTRY/DISPLAY/MODEL,
# GODOT_OUTBID_MAIL_ID and GODOT_OUTBID_CAPTURE. No mail injection or direct requests.
const OUTBID_BODY := "You have been outbid on Linen Cloth. Your 1g 23s 45c has been refunded."

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var username := OS.get_environment("GODOT_TEST_ACCOUNT")
	var mail_id := OS.get_environment("GODOT_OUTBID_MAIL_ID").to_int()
	if not endpoint.begins_with("127.0.0.1:") or not username.begins_with("fb_") or mail_id <= 0:
		fail("Owned private endpoint/account and authentic Outbid mail required")
		return
	var recipient: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(recipient)
	var error = recipient.connect_account(endpoint, username, "fbtest", false)
	if error != "":
		fail("Connect: " + error)
		return
	if not await wait_recipient_screen(recipient, "CharacterSelect"):
		return
	var enter = recipient.find_child("EnterWorld", true, false) as Control
	if enter == null:
		fail("Enter World missing")
		return
	await click(enter)
	if not await wait_recipient_screen(recipient, "InWorld"):
		return
	await create_timer(15.0).timeout
	if not await open_mailbox(recipient) or not await select_mail(recipient, mail_id):
		return
	var body := recipient.find_child("OpenMailBodyText", true, false) as Label
	if body == null or body.text != OUTBID_BODY:
		fail("Authentic denomination body not received")
		return
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	var capture := OS.get_environment("GODOT_OUTBID_CAPTURE")
	if capture.is_empty() or root.get_texture().get_image().save_png(capture) != OK:
		fail("Cannot capture rendered letter")
		return
	var line_count := body.get_line_count()
	var visible_lines := body.get_visible_line_count()
	print("OUTBID_BODY_LAYOUT lines=", line_count, " visible=", visible_lines, " rect=", body.get_global_rect(), " text=", body.text)
	if line_count < 2 or visible_lines != line_count:
		fail("Outbid letter must display all wrapped lines, including 23s 45c; got lines=%s visible=%s" % [line_count, visible_lines])
		return
	if recipient.mail_state().money != 1000000:
		fail("Refunded balance is not exactly 1000000 copper")
		return
	print("PASS: authentic Outbid letter wraps without hidden lines and refund is exact")
	recipient.free()
	quit(0)

func wait_recipient_screen(recipient: Node, wanted: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = recipient.account_state()
		if state.screen == wanted and state.reply_received and not state.assets_starting:
			return true
	fail("Timed out waiting for recipient screen " + wanted)
	return false
