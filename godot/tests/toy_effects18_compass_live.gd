extends "res://tests/toy_effects14_live.gd"
# Actual private bag learning, journal dispatch and native owning-player facing.
func proof_cases() -> Array:
	return [[45984, 64385, 0, "Unusual Compass"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var search := control("ToyBoxSearchBox") as LineEdit
	search.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "source compass search"): return
	var before: float = client.account_state().local_player_facing
	var before_position: Vector3 = client.account_state().local_server_position
	await capture("compass-before")
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return client.spells_state().casting == 64385, "authored compass cast"): return
	if not await wait_until(func(): return client.spells_state().casting == 0, "source 2000ms completion"): return
	receipts.append({"phase": "compass-cast-completed", "before_yaw": before, "after_yaw": client.account_state().local_player_facing, "spells": client.spells_state(), "toys": client.toybox_state()})
	write_receipts()
	if not await wait_until(func(): return absf(angle_difference(before, float(client.account_state().local_player_facing))) > 0.0001, "owning-client compass facing", 10000): return
	var after: float = client.account_state().local_player_facing
	await create_timer(1.0).timeout
	if absf(angle_difference(after, float(client.account_state().local_player_facing))) > 0.0001: fail("ordinary idle input overwrote forced facing"); return
	if client.account_state().local_server_position.distance_to(before_position) > 0.05: fail("compass moved player"); return
	receipts.append({"phase": "source-compass-facing", "before_yaw": before, "after_yaw": after, "before_position": before_position, "after_position": client.account_state().local_server_position})
	await capture("compass-after")

func fail(message: String) -> void:
	if client != null:
		receipts.append({"phase": "compass-failure", "error": message, "account": client.account_state(), "spells": client.spells_state()})
		write_receipts()
	super.fail(message)
