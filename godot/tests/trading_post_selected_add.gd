extends "res://tests/trading_post_hover_add.gd"
# Fixture-only selected-texture pixel oracle; never infer RED from blend fields.
var selected_samples: Array = []
var selected_ids: Array = []
var cart_cleared_selection := false
var cart_restored_selection := false

func _initialize() -> void:
	Engine.max_fps = 60
	gate = Completion.new(self, 10)
	call_deferred("run_fixture")

func run_cases() -> bool:
	if not gate.require(OS.get_environment("TP_SELECTED_ADD_OPT_IN") == "1","explicit selected ADD pixel proof opt-in required"): return false
	if not gate.step(await connect_private(),"private account/world/vendor"): return false
	baseline = client.trading_post_state().duplicate(true)
	appearances = Array(client.tooltip_state().appearance_collection)
	appearances.sort()
	origin = client.account_state().local_player_position
	if not gate.require(baseline.cart.is_empty() and int(baseline.frozen) > 0,"empty cart/genuine reservation required"): return false
	if not await only_type(1): return false
	for entry in client.trading_post_state().rows:
		var fields := str(entry).split("|")
		if fields[1] == "1" and int(fields[0]) != int(baseline.frozen):
			selected_ids.append(int(fields[0]))
			if selected_ids.size() == 2: break
	if not gate.require(selected_ids.size() == 2,"two available ordinary products required"): return false
	for id in selected_ids:
		if not await collect_selected_pixels(int(id)):
			restore_counterfactual()
			return false
		selected_samples.append(proof.duplicate(true))
	if not await collect_cart_selection_controls(): return false
	if not await restore_original_state(): return false
	if not write_selected_receipt(): return false
	# Restore every counterfactual/local/authority owner before either RED or GREEN.
	var cases := [
		[all_samples("source_visible"),"authored selected textures contain visible source texels"],
		[all_samples("backdrop_stable"),"stable selected-texture reference backdrop"],
		[all_samples("actual_matches_add"),"actual selected pixels must equal source ADD reference"],
		[all_samples("add_alpha_distinguishable"),"selected authored pixels distinguish ADD from MIX"],
		[all_samples("original_restored"),"exact original selected material restoration"],
		[selected_samples.size() == 2 and selected_ids[0] != selected_ids[1],"two ordinary selected products sampled"],
		[bool(selected_samples[1].previous_selected_absent),"second selection removes first selected art"],
		[cart_cleared_selection,"cart owner removes ordinary selected art"],
		[cart_restored_selection,"cart return restores the same ordinary selected owner"],
		[bool(proof.authority_restored),"authority/local/HUD/world conserved without transactions"]
	]
	for result in cases:
		if not gate.require(bool(result[0]),str(result[1])): return false
		if not gate.case_done(): return false
	return true

func all_samples(key: String) -> bool:
	for sample in selected_samples:
		if not bool(sample[key]): return false
	return selected_samples.size() == 2

func collect_selected_pixels(id: int) -> bool:
	product = id
	if not await click_checked("PerksProgramProduct%d" % product): return false
	await motion(Vector2(1850,80))
	await frames(20)
	var state: Dictionary = client.trading_post_state()
	if not gate.require(int(state.selected) == product and not str(state.selected_name).is_empty(),"selected metadata-bearing ordinary product"): return false
	var owner := control("PerksProgramSelected%d" % product)
	if not gate.require(owner != null and selected_area("PerksProgramSelected%d" % product) > 0.,"native selected texture owner"): return false
	row_rect = owner.get_global_rect()
	backdrop = ColorRect.new()
	backdrop.name = "SelectedAddDiagnosticBackdrop"
	backdrop.color = Color(0.2,0.3,0.4,1.)
	backdrop.size = row_rect.size
	backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	# Same canvas, directly beneath this existing selected texture; no hover overlay.
	backdrop.z_as_relative = true
	backdrop.z_index = -1
	owner.add_child(backdrop)
	overlay = owner.get_node_or_null("Parts/Part0") as TextureRect
	if not gate.require(overlay != null and overlay.texture != null,"existing decoded selected texture required"): return false
	original_material = overlay.material
	var source := overlay.texture.get_image()
	if not gate.require(source != null and not source.is_empty(),"selected atlas readable"): return false
	if not gate.require(source.save_png(shots.path_join("%s-selected-add-%d-source.png" % [skin,product])) == OK,"save selected atlas source"): return false
	proof = {"product":product,"item_id":state.item_id,"name":state.selected_name,"row_rect":[row_rect.position.x,row_rect.position.y,row_rect.size.x,row_rect.size.y],"source_visible":has_visible_source(source),"previous_selected_absent":selected_samples.is_empty() or selected_area("PerksProgramSelected%d" % int(selected_ids[0])) == 0.}
	var actual := await capture_blend("actual-selected")
	var add := CanvasItemMaterial.new()
	add.blend_mode = CanvasItemMaterial.BLEND_MODE_ADD
	var reference := await capture_blend("reference-add",add)
	var stable := await capture_blend("reference-add-stable",add)
	var alpha := CanvasItemMaterial.new()
	alpha.blend_mode = CanvasItemMaterial.BLEND_MODE_MIX
	var mixed := await capture_blend("reference-alpha",alpha)
	var restored := await capture_blend("actual-restored")
	proof.actual_add_different_pixels = count_pixel_differences(actual,reference)
	proof.actual_alpha_different_pixels = count_pixel_differences(actual,mixed)
	proof.add_alpha_different_pixels = count_pixel_differences(reference,mixed)
	proof.restored_actual_different_pixels = count_pixel_differences(actual,restored)
	proof.actual_matches_add = int(proof.actual_add_different_pixels) == 0
	proof.add_alpha_distinguishable = int(proof.add_alpha_different_pixels) >= 32
	proof.original_restored = int(proof.restored_actual_different_pixels) == 0
	proof.backdrop_stable = count_pixel_differences(reference,stable) == 0
	restore_counterfactual()
	await frames(20)
	await capture_blend("normal-restored")
	return true

func capture_blend(label: String, reference_material: Material = null) -> Image:
	await frames(4)
	await RenderingServer.frame_pre_draw
	var current: TextureRect
	var saved: Material
	if reference_material != null:
		current = control("PerksProgramSelected%d" % product).get_node("Parts/Part0") as TextureRect
		saved = current.material
		current.material = reference_material
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if reference_material != null: current.material = saved
	gate.require(image.save_png(shots.path_join("%s-selected-add-%d-%s.png" % [skin,product,label])) == OK,"save selected blend PNG")
	var crop := Rect2i(int(row_rect.position.x)+3,int(row_rect.position.y)+3,int(row_rect.size.x)-6,int(row_rect.size.y)-6)
	return image.get_region(crop)

func collect_cart_selection_controls() -> bool:
	if not await click_checked("PerksProgramAddToCartButton"): return false
	if not await click_checked("PerksProgramViewCartButton"): return false
	await motion(Vector2(1850,80))
	await frames(20)
	var state: Dictionary = client.trading_post_state()
	cart_cleared_selection = bool(state.cart_visible) and int(state.selected) == -1
	for id in selected_ids:
		cart_cleared_selection = cart_cleared_selection and selected_area("PerksProgramSelected%d" % int(id)) == 0.
	await capture_blend("cart-cleared")
	if not await click_checked("PerksProgramCartCloseButton"): return false
	await motion(Vector2(1850,80))
	await frames(20)
	state = client.trading_post_state()
	cart_restored_selection = not bool(state.cart_visible) and int(state.selected) == product and selected_area("PerksProgramSelected%d" % product) > 0.
	await capture_blend("cart-return")
	if not await click_checked("PerksProgramRemoveFromCartButton"): return false
	return gate.require(client.trading_post_state().cart.is_empty(),"local cart cleanup before authority restoration")

func write_selected_receipt() -> bool:
	var file := FileAccess.open(receipts.path_join(skin+"-selected-add-result.json"),FileAccess.WRITE)
	if not gate.require(file != null,"write selected ADD pixel receipt"): return false
	proof.selected_samples = selected_samples
	proof.cart_cleared_selection = cart_cleared_selection
	proof.cart_restored_selection = cart_restored_selection
	proof.pixel_tolerance = 1.5/255.
	proof.oracle = "existing selected texture pixels versus same-atlas forced native ADD/MIX; pointer off row; original restoration before verdict"
	file.store_string(JSON.stringify(proof,"\t"))
	file.close()
	return true
