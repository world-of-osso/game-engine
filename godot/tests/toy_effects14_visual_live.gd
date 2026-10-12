extends "res://tests/toy_effects14_live.gd"
# Source-audited aura4 art: normal player model stays unchanged.
func proof_cases() -> Array:
	return [[116115, 170869, 519369, "Blazing Wings"]]

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	var model := int(sample[2])
	await learn_toy(item)
	if failed or not learned_toy(item): return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "search " + str(item)): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return has_aura(spell) and kit_model(spell, model), "source wing model " + str(model), 120000): return
	if not native_display(-1): fail("attachment replaced native player model"); return
	await capture("toy%d-model%d" % [item, model])
	await cancel_aura(spell)
	if failed: return
	if not await wait_until(func(): return not has_aura(spell) and not kit_model(spell, model), "wing removal", 30000): return
	if not native_display(-1): fail("attachment removal changed native model"); return
	await capture("toy%d-removed" % item)

func kit_model(spell: int, model: int) -> bool:
	var local_id := int(client.account_state().local_player_id)
	for active in client.spell_visuals_state().active:
		if int(active.spell) == spell and int(active.model) == model and int(active.unit) == local_id:
			return true
	return false
