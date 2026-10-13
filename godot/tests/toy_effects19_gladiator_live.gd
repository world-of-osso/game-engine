extends "res://tests/toy_effects18_castkit_live.gd"
# Actual Alliance character learning, persistent banner aura, source back model,
# cancellation and native model restoration. Horde variants have core/server proof.
func proof_cases() -> Array:
	return [[119220, 176899, [1332368], "Alliance Gladiator's Banner"]]

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	await learn_toy(item)
	if failed or not learned_toy(item): return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	if not await wait_until(func(): return control("ToyBoxSearchBox") != null, "Toy Box"): return
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "Gladiator banner search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return has_aura(spell) and kit_model(spell, sample[2]) != 0, "source back banner", 120000): return
	if not native_display(-1): fail("banner replaced native model"); return
	receipts.append({"phase": "gladiator-banner", "item": item, "spell": spell, "model": kit_model(spell, sample[2])})
	client.set_camera_orbit(PI / 2.0, -0.3, 8.0)
	await capture("gladiator-toy%d-back-banner" % item)
	await cancel_aura(spell)
	if failed: return
	if not await wait_until(func(): return not has_aura(spell) and kit_model(spell, sample[2]) == 0 and native_display(-1), "banner cancellation", 30000): return
	await capture("gladiator-toy%d-restored" % item)
