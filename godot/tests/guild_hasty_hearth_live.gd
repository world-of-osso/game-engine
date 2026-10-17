extends "res://tests/guild_perks_live.gd"

var officer_dir := ""
var hasty_mode := ""
var phase := 0
var casts_completed := 0
var expected_recharge := 0

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	diagnostics = OS.get_environment("GUILD_PERKS_DIAGNOSTICS")
	officer_dir = OS.get_environment("GUILD_PERKS_OFFICER_DIR")
	hasty_mode = OS.get_environment("GUILD_HASTY_MODE")
	assert(OS.get_environment("GODOT_TEST_SERVER") == "127.0.0.1:5891", "Private5891 only")
	assert(OS.get_environment("BANK_ACCOUNT") == "fb_guildcompletion_perkspeer", "Private ordinary member only")
	assert(hasty_mode in ["baseline", "member"], "Explicit fresh-process charge scenario required")
	# Actual Human458619 supplies +1 charge/-300000ms, retained throughout.
	expected_recharge = 1500000 if hasty_mode == "baseline" else 600000
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	role = "hasty-entry"
	if not await connect_private_account() or not await enter_world("Gcperkspeer"):
		return
	if not mark_officer("member-ready.go"):
		return
	if not await wait_until(func(): return FileAccess.file_exists(officer_dir.path_join("riding-ready.go")), "actual private actor setup"):
		return
	if not await wait_until(func(): return float(perk_observation().health) > 0.0, "actual living private actor after fixture recovery"):
		return
	if not 458619 in Array(perk_observation().known_spells):
		fail("Actual Human No Place Like Home racial must remain known")
		return
	for skin in ["Modern", "Forever"]:
		phase += 1
		if not await select_skin(skin):
			return
		role = skin.to_lower() + "-hasty-" + hasty_mode
		await close_guild()
		if not known_perks().is_empty():
			fail("Each invitation must start with the actual nonmember")
			return
		if hasty_mode == "baseline":
			if not await cast_actual_hearthstone():
				return
		if not mark_officer("invite-%d.go" % phase):
			return
		if not await wait_until(func(): return shown("GuildInviteFrameJoinButton"), "actual officer invitation"):
			return
		await press("GuildInviteFrameJoinButton")
		if not await wait_until(func(): return known_perks().size() == 5, "actual membership passive grants"):
			return
		await close_guild()
		if hasty_mode == "member":
			if not await cast_actual_hearthstone():
				return
		if not await prove_pending("member-pending"):
			return
		if not mark_officer("remove-%d.go" % phase):
			return
		if not await wait_until(func(): return known_perks().is_empty(), "real officer removal revokes Hasty Hearth"):
			return
		if not await prove_pending("removed-pending"):
			return
	mark_officer("done.go")
	print("PASS GUILD_HASTY_HEARTH_NATIVE mode=%s recharge_ms=%d both_skins=true actual_item6948=true actual_spell8690=true actual_casts=%d" % [hasty_mode, expected_recharge, casts_completed])
	client.free()
	quit(0)

func open_hearth_backpack() -> bool:
	if not shown("ContainerFrame0Slot0"):
		await tap(KEY_B)
	return await wait_until(func(): return hearth_button() != null and hearth_button().is_visible_in_tree(), "actual Hearthstone in open backpack")

func hearth_button() -> Control:
	for entry in client.merchant_state().bags:
		if entry.item_id == 6948 and entry.bag == 0:
			return control("ContainerFrame0Slot%d" % entry.slot)
	return null

func cast_actual_hearthstone() -> bool:
	if not await open_hearth_backpack():
		return false
	var successes_before: int = Array(client.spells_state().cast_success).count(8690)
	var expected_current: int = 2 - phase
	await click_at(hearth_button().get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	if not await wait_until(func():
		var charge: Variant = perk_observation().hearth_charges
		return charge != null and charge.current == expected_current and charge.max == 2 and Array(client.spells_state().cast_success).count(8690) > successes_before, "real item6948 completes spell8690 and consumes charge2309"):
		return false
	casts_completed += 1
	return await prove_pending("actual-cast")

func prove_pending(label: String) -> bool:
	var observation: Dictionary = perk_observation()
	var charge: Variant = observation.hearth_charges
	if charge == null or charge.current != 2 - casts_completed or charge.max != 2:
		return fail("Actual pending Hearthstone charge missing: " + str(charge))
	if abs(float(charge.recharge_ms) - expected_recharge) > 2.0:
		return fail("Guild change retimed consumed Hearthstone duration: " + str(charge))
	# Bounded native run observes the live clock, not injected/shortened recovery.
	if float(charge.remaining_ms) < expected_recharge - 300000 or float(charge.remaining_ms) > expected_recharge:
		return fail("Consumed Hearthstone remaining time changed unexpectedly: " + str(charge))
	if not 458619 in Array(observation.known_spells):
		return fail("Guild reconciliation removed the actual Human racial")
	write_json(role + "-" + label + ".json", {"owner": observation, "spells": client.spells_state(), "bags": client.merchant_state().bags})
	if not await open_hearth_backpack():
		return false
	await hover_at(hearth_button().get_global_rect().get_center())
	await capture(label)
	return true

func fail(message: String) -> bool:
	if is_instance_valid(client):
		write_json(role + "-spell-failure-state.json", client.spells_state())
		write_json(role + "-item-failure-state.json", client.merchant_state())
	return super.fail(message)

func mark_officer(name: String) -> bool:
	var file := FileAccess.open(officer_dir.path_join(name), FileAccess.WRITE)
	if file == null:
		return fail("Cannot publish actual officer coordination marker " + name)
	file.store_string("go")
	file.close()
	return true
