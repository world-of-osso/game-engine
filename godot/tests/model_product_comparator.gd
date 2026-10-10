extends "res://tests/model_product_isolation.gd"

# Independent Retail comparator; never substitutes for the two-display acceptance.
func run() -> void:
	await run_displays([21774])
