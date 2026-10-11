extends "res://tests/toy_live2.gd"
# Full production ordering: initial catalog, then repeated bag learn/use/cancel.
# No journal reopen between the bag action and its learning assertion.
func learn_sample(item: int) -> void:
	await learn_toy(item)
	if not failed:
		print("TOYLEARN_SEQUENCE item=", item, " learned=", learned_toy(item))
