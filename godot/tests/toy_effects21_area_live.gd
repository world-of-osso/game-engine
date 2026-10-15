extends "res://tests/toy_effects20_war_live.gd"
# Actual Eagger Basket bag learning and journal activation; real ADT areas9/87.
# Admin travel changes position only, never aura or cooldown history.
func proof_cases() -> Array:
	return [[188698, 61810, [], "Eagger Basket"]]

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	await learn_toy(item)
	if failed or not learned_toy(item): return
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, "-8949.95", "-132.493", "83.5312"]): return
	await create_timer(1.0).timeout
	await open_source_toy(str(sample[3]))
	if failed: return
	await click("ToySpellButton1")
	if not await wait_until(func(): return client.toybox_state().error == "Requires area group1103", "native source area refusal"): return
	if has_aura(spell) or remaining_cooldown(item) > 0.0: fail("rejected source area started aura or cooldown"); return
	await capture("area21-northshire-refused")
	# content_creature20280725 Cow2442; parsed778032 MCNK Area87.
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, "-9480.94", "-38.5858", "59.6236"]): return
	await create_timer(1.0).timeout
	await click("ToySpellButton1")
	if not await wait_until(func(): return has_aura(spell) and remaining_cooldown(item) > 0.0, "valid Goldshire activation"): return
	var before := remaining_cooldown(item)
	receipts.append({"phase": "allowed-area", "remaining": before, "toys": client.toybox_state(), "auras": client.aura_state()})
	await click("CollectionsMicroButton")
	await capture("area21-goldshire-permitted")
	# TDB1210 creature305516/41478, actual ADT777997 Kharanos Area131.
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, "-5437.77", "-437.216", "398.038"]): return
	await create_timer(1.0).timeout
	if not has_aura(spell): fail("moving to another permitted member removed the aura"); return
	receipts.append({"phase": "second-permitted-area", "remaining": remaining_cooldown(item), "auras": client.aura_state()})
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, "-8949.95", "-132.493", "83.5312"]): return
	if not await wait_until(func(): return not has_aura(spell), "area exit interrupted aura"): return
	var after := remaining_cooldown(item)
	if after <= 0.0 or after > before: fail("area exit lost or reset cooldown history"); return
	receipts.append({"phase": "area-exit", "remaining_before": before, "remaining_after": after, "toys": client.toybox_state(), "auras": client.aura_state()})
	await capture("area21-northshire-removed-cooldown-kept")
	print("PASS toy-effects14 actual area refusal/activation/exit/cooldown ", skin)

func open_source_toy(name: String) -> void:
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(name)
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == name, "source toy search"): return

func remaining_cooldown(item: int) -> float:
	for row in client.toybox_state().learned:
		if int(row.item_id) == item: return float(row.cooldown)
	return 0.0
