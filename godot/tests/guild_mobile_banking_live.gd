extends "res://tests/guild_perks_live.gd"

# Real private-server timing. No shortened lifetime, injected clock or cooldown reset.
var officer_dir := ""
var invite_number := 0
var removal_number := 0
var chest_id := 0
var cast_at := 0

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	diagnostics = OS.get_environment("GUILD_PERKS_DIAGNOSTICS")
	officer_dir = OS.get_environment("GUILD_PERKS_OFFICER_DIR")
	assert(OS.get_environment("GODOT_TEST_SERVER") == "127.0.0.1:5891")
	assert(OS.get_environment("BANK_ACCOUNT") == "fb_guildcompletion_perkspeer")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	role = "mobile-entry"
	if not await connect_private_account() or not await enter_world("Gcperkspeer"):
		return
	mark_officer("member-ready.go")
	if not await wait_until(func(): return FileAccess.file_exists(officer_dir.path_join("actor-ready.go")), "living grounded private actor setup"):
		return
	if not await wait_until(func(): return not perk_observation().startup_loading and float(perk_observation().health) > 0.0, "world/model startup"):
		return
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 14.0)
	if not await join_actual_guild():
		return
	for skin in ["Modern", "Forever"]:
		if not await select_skin(skin):
			return
		role = skin.to_lower() + "-mobile-banking"
		await close_guild()
		if not await prove_chest_cycle():
			return
	# Final real recast proves readiness after the Forever two-hour interval;
	# Modern readiness was proved by the next skin's ordinary cast.
	var successes: int = Array(client.spells_state().cast_success).count(83958)
	client.use_spell(83958)
	if not await wait_until(func(): return Array(client.spells_state().cast_success).count(83958) > successes and find_new_chest(), "actual recast accepted after Forever2h"):
		return
	await capture("after-two-hours-recast")
	mark_officer("done.go")
	print("PASS GUILD_MOBILE_BANKING_NATIVE both_skins=true actual_cast=true member_bank=true nonmember_refused=true lifetime300s=true cooldown7200s=true real_wall_clock=true")
	client.free()
	quit(0)

func mark_officer(name: String) -> void:
	var file := FileAccess.open(officer_dir.path_join(name), FileAccess.WRITE)
	assert(file != null)
	file.store_string("actual native stage")
	file.close()

func join_actual_guild() -> bool:
	invite_number += 1
	mark_officer("invite-%d.go" % invite_number)
	if not await wait_until(func(): return shown("GuildInviteFrameJoinButton"), "actual officer invitation"):
		return false
	await press("GuildInviteFrameJoinButton")
	return await wait_until(func(): return 83958 in Array(client.spells_state().known), "membership grants Mobile Banking")

func chest_node() -> Node3D:
	for node in client.find_children("*", "Node3D", true, false):
		if node.has_meta("game_object_server_id") and int(node.get_meta("game_object_server_id")) == chest_id:
			return node
	return null

func find_new_chest() -> bool:
	for node in client.find_children("*", "Node3D", true, false):
		if node.has_meta("game_object_name") and str(node.get_meta("game_object_name")) == "Guild Chest" and node.has_meta("game_object_server_id"):
			chest_id = int(node.get_meta("game_object_server_id"))
			return true
	return false

func prove_chest_cycle() -> bool:
	var successes: int = Array(client.spells_state().cast_success).count(83958)
	if str(client.use_spell(83958)) != "":
		return fail("Native action rejected known perk")
	if not await wait_until(func(): return Array(client.spells_state().cast_success).count(83958) > successes and find_new_chest(), "actual83958 cast and rendered206602"):
		return false
	cast_at = Time.get_ticks_msec()
	var clock: Variant = perk_observation().mobile_cooldown
	if clock == null or abs(float(clock.duration) - 7200.0) > 0.01:
		return fail("Server cooldown is not user7200s: " + str(clock))
	await capture("cast-chest")
	if not await click_pickable(func(node): return node.has_meta("game_object_server_id") and int(node.get_meta("game_object_server_id")) == chest_id, "game_object_server_id"):
		return false
	if not await wait_until(func(): return shown("GuildBankFrame") and client.bank_state().guild_open and int(client.bank_state().guild_object) == chest_id, "actual mobile guild-bank contents"):
		return false
	write_json(role + "-bank.json", {"at_ms": Time.get_ticks_msec(), "cast_at_ms": cast_at, "chest": chest_id, "bank": client.bank_state(), "owner": perk_observation(), "spells": client.spells_state()})
	await capture("member-bank")
	removal_number += 1
	mark_officer("remove-%d.go" % removal_number)
	if not await wait_until(func(): return not (83958 in Array(client.spells_state().known)) and not client.bank_state().guild_open, "actual removal revokes knowledge and closes mobile vault"):
		return false
	if not await click_pickable(func(node): return node.has_meta("game_object_server_id") and int(node.get_meta("game_object_server_id")) == chest_id, "game_object_server_id"):
		return false
	if not await wait_until(func(): return invalid_target_error() and not shown("GuildBankFrame"), "nonmember UseGameObject refusal"):
		return false
	write_json(role + "-nonmember-refused.json", {"at_ms": Time.get_ticks_msec(), "chest": chest_id, "bank": client.bank_state(), "owner": perk_observation()})
	await capture("nonmember-refused")
	if not await join_actual_guild() or not await refuse_recast("rejoined-cooldown"):
		return false
	if not await wait_for_real_expiry():
		return false
	await capture("expired")
	print("MOBILE_FIVE_MINUTE_PASS role=", role, " actual_elapsed_ms=", Time.get_ticks_msec() - cast_at)
	# Both skins observe one hour and the two-hour boundary using actual elapsed
	# server time. Reduce render load while no interaction is due; never time_scale.
	if not await wait_until_elapsed(3600000) or not await refuse_recast("one-hour-refused"):
		return false
	if not await wait_until_elapsed(7195000) or not await refuse_recast("before-two-hours-refused"):
		return false
	if not await wait_until_elapsed(7201000):
		return false
	var remaining: Variant = perk_observation().mobile_cooldown
	if remaining != null and float(remaining.remaining) > 0.1:
		return fail("Server clock still pending after real2h: " + str(remaining))
	write_json(role + "-two-hour-ready.json", {"at_ms": Time.get_ticks_msec(), "cast_at_ms": cast_at, "owner": perk_observation()})
	await capture("two-hour-ready")
	print("MOBILE_TWO_HOUR_PASS role=", role, " actual_elapsed_ms=", Time.get_ticks_msec() - cast_at)
	return true

func invalid_target_error() -> bool:
	var host := client.get_node_or_null("UIErrors")
	if host == null:
		return false
	for label in host.find_children("*", "Label", true, false):
		if label.is_visible_in_tree() and str(label.text) == "Invalid target":
			return true
	return false

func refuse_recast(label: String) -> bool:
	var before: int = int(client.spells_state().cast_failure_seq)
	var count_before: int = Array(client.spells_state().cast_success).count(83958)
	client.use_spell(83958)
	if not await wait_until(func(): return int(client.spells_state().cast_failure_seq) > before, "actual server cooldown refusal"):
		return false
	var rows: Array = client.spells_state().cast_failure_events
	var row: Dictionary = rows.back()
	if int(row.spell_id) != 83958 or row.reason != "OnCooldown" or Array(client.spells_state().cast_success).count(83958) != count_before:
		return fail("Wrong recast refusal: " + str(row))
	write_json(role + "-" + label + ".json", {"at_ms": Time.get_ticks_msec(), "cast_at_ms": cast_at, "owner": perk_observation(), "failure": row})
	await capture(label)
	return true

func wait_for_real_expiry() -> bool:
	var deadline := cast_at + 330000
	while Time.get_ticks_msec() < deadline:
		await create_timer(1.0).timeout
		if chest_node() == null:
			var elapsed := Time.get_ticks_msec() - cast_at
			if elapsed < 295000:
				return fail("Guild chest disappeared before installed5min: " + str(elapsed))
			write_json(role + "-expiry.json", {"at_ms": Time.get_ticks_msec(), "cast_at_ms": cast_at, "chest": chest_id, "elapsed_ms": elapsed, "bank": client.bank_state()})
			return true
	return fail("Guild chest survived installed5min")

func wait_until_elapsed(milliseconds: int) -> bool:
	Engine.max_fps = 2
	while Time.get_ticks_msec() - cast_at < milliseconds:
		await create_timer(1.0).timeout
		if float(perk_observation().health) <= 0.0:
			Engine.max_fps = 30
			return fail("Private actor died during actual cooldown observation")
	Engine.max_fps = 30
	await wait_frames(10)
	return true
