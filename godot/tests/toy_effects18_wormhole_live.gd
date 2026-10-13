extends "res://tests/toy_effects14_live.gd"
# Actual bag learning and journal use; the server sends the player to one authored Pandaria point.
const PANDARIA := 870

func proof_cases() -> Array:
	return [[87215, 126755, 0, "Wormhole Generator: Pandaria"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	var origin_map := int(client.account_state().world_map_id)
	if origin_map == PANDARIA: fail("proof must start outside Pandaria"); return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text("Wormhole Generator: Pandaria")
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == "Wormhole Generator: Pandaria", "source wormhole search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return int(client.account_state().world_map_id) == PANDARIA and client.account_state().screen == "InWorld", "authored Pandaria transfer", 240000): return
	if not await wait_until(func(): return native_display(-1), "native player model in Pandaria", 120000): return
	receipts.append({"phase": "source-wormhole-transfer", "from_map": origin_map, "account": client.account_state()})
	await capture("toy87215-pandaria")
