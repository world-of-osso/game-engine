extends "res://tests/toy_effects24_tome_live.gd"
# Learning an authentic unsupported toy is allowed; no activation or gameplay claim.
func proof_cases() -> Array:
	return [[90067]]

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	if learned_toy(item): fail("learning diagnostic requires fresh owned item"); return
	await learn_toy(item)
	if failed: return
	receipts.append({"phase": "learning-delivered", "toys": client.toybox_state(), "bags": client.merchant_state().bags})
	write_receipts()
	print("PASS toy-effects14 actual unsupported toy learning update delivered ", skin)
