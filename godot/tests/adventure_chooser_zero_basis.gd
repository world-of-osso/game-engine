extends "res://tests/adventure_remornia_attribution.gd"
# Count the actual Model-tab interval, retaining all pre-window events separately.
var basis_before_model_tab := -1

func click(name: String) -> bool:
	if name == "EncounterJournalPage3":
		basis_before_model_tab = observer.count_basis_errors()
	return await super.click(name)

func run() -> void:
	tag_phase("client-load-through-world-and-Journal")
	completion = load("res://tests/completion.gd").new(self, 1)
	var result := await run_plain_animation()
	observe_frame()
	var total := observer.count_basis_errors()
	var model_tab_errors := total - basis_before_model_tab
	print("JOURNAL_CHOOSER_ZERO_BASIS ", JSON.stringify({"basis_before_model_tab":basis_before_model_tab,"basis_through_stable213":total,"model_tab_basis_errors":model_tab_errors,"all_logged_events":error_count,"source_case":5204,"skin":OS.get_environment("ADVENTURE_GUIDE_EXPECTED_SKIN")}))
	if not completion.require(basis_before_model_tab >= 0 and model_tab_errors == 0, "Physical Model tab caused%s Basis quaternion errors (pre-window%s)" % [model_tab_errors,basis_before_model_tab]):
		return
	completion.finish(result)
