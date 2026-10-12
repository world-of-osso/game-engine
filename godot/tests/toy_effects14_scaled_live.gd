extends "res://tests/toy_effects14_live.gd"
# Actual bag/journal activation of source-scaled stationary scenery on private5911.
func proof_cases() -> Array:
	return [[170476, 304620, 92605, "Underlight Sealamp"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(int(sample[0]))
	if failed: return
	await click("CollectionsMicroButton")
	await click("CollectionsJournalTab3")
	await click("ToyBoxSearchBox")
	var edit := control("ToyBoxSearchBox") as LineEdit
	edit.select_all()
	await type_text(str(sample[3]))
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == str(sample[3]), "source scaled toy search"): return
	await click("ToySpellButton1")
	await click("CollectionsMicroButton")
	if not await wait_until(func(): return lamp() != null, "source lamp native meshes", 120000): return
	var placed := lamp()
	if abs(placed.scale.x - 0.35) > 0.00001: fail("authored display scale.35 was dropped"); return
	if control("PetFrame") != null: fail("scenery acquired the combat pet frame"); return
	var model := placed.find_children("NpcModel", "Node3D", true, false)[0] as Node3D
	receipts.append({"phase": "scaled-source-lamp", "entry": 156086, "display": 92605, "source_display_scale": 0.35, "replicated_root_scale": [placed.scale.x, placed.scale.y, placed.scale.z], "native_visual_scale": [model.scale.x, model.scale.y, model.scale.z], "meshes": model.find_children("*", "MeshInstance3D", true, false).size()})
	await capture("toy170476-display92605-scale035")

func lamp() -> Node3D:
	for unit in client.find_children("Underlight Sealamp", "Node3D", true, false):
		if not unit.find_children("NpcModel", "Node3D", true, false).is_empty() and not unit.find_children("*", "MeshInstance3D", true, false).is_empty(): return unit
	return null

func capture(label: String) -> void:
	await super.capture("scaled-" + label)
