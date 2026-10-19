extends "res://tests/trading_post_hover_add.gd"
# Committed reservation only; no freeze/clear transaction or pending-candidate workflow.
var ordinary := 0
var ordinary_before := false
var ordinary_after := false
var frozen_return := false
var cart_cleared_selection := false
var cart_restored_selection := false

func _initialize() -> void:
	Engine.max_fps = 60
	gate = Completion.new(self, 9)
	call_deferred("run_fixture")

func run_cases() -> bool:
	if not gate.require(OS.get_environment("TP_FROZEN_SELECTED_ADD_OPT_IN") == "1","explicit frozen selected ADD pixel proof opt-in required"): return false
	if not gate.step(await connect_private(),"private account/world/vendor"): return false
	baseline = client.trading_post_state().duplicate(true)
	appearances = Array(client.tooltip_state().appearance_collection)
	appearances.sort()
	origin = client.account_state().local_player_position
	if not gate.require(baseline.cart.is_empty() and int(baseline.frozen) > 0,"empty cart and genuine existing reservation required"): return false
	product = int(baseline.frozen)
	for entry in baseline.rows:
		var fields := str(entry).split("|")
		if fields[1] == "1" and int(fields[0]) != product:
			ordinary = int(fields[0])
			break
	if not gate.require(ordinary > 0,"observed available ordinary cart product required"): return false
	if not await click_checked("PerksProgramProduct%d" % ordinary): return false
	await motion(Vector2(1850,80))
	await frames(20)
	ordinary_before = selected_area("PerksProgramFrozenSelected") == 0.
	if not await collect_frozen_pixels():
		restore_counterfactual()
		return false
	if not await collect_selection_controls(): return false
	if not await collect_cart_controls(): return false
	if not await restore_original_state(): return false
	if not write_frozen_receipt(): return false
	# Authority/local/HUD/counterfactual restoration precedes either RED or GREEN.
	var cases := [
		[bool(proof.source_visible),"authored frozen selected texture contains visible source texels"],
		[bool(proof.backdrop_stable),"stable frozen selected reference backdrop"],
		[int(proof.actual_add_different_pixels) == 0,"actual frozen selected pixels must equal source ADD reference"],
		[int(proof.add_alpha_different_pixels) >= 32,"authored frozen selected pixels distinguish ADD from MIX"],
		[int(proof.restored_actual_different_pixels) == 0,"exact original frozen selected material restoration"],
		[ordinary_before and ordinary_after and frozen_return,"ordinary/frozen/ordinary selected-owner controls"],
		[cart_cleared_selection,"cart owner clears frozen selected art"],
		[cart_restored_selection,"cart return restores committed frozen selected owner"],
		[bool(proof.authority_restored),"authority/local/HUD/world conserved without transactions"]
	]
	for result in cases:
		if not gate.require(bool(result[0]),str(result[1])): return false
		if not gate.case_done(): return false
	return true

func select_committed_frozen() -> bool:
	if not await click_checked("PerksProgramFrozenSlot"): return false
	await motion(Vector2(1850,80))
	await frames(20)
	var state: Dictionary = client.trading_post_state()
	return gate.require(int(state.frozen) == product and int(state.selected) == product and not str(state.selected_name).is_empty(),"selected genuine committed reservation, not fabricated metadata")

func collect_frozen_pixels() -> bool:
	if not await select_committed_frozen(): return false
	var owner := control("PerksProgramFrozenSelected")
	if not gate.require(owner != null and selected_area("PerksProgramFrozenSelected") > 0.,"existing frozen selected texture owner"): return false
	row_rect = owner.get_global_rect()
	backdrop = ColorRect.new()
	backdrop.name = "FrozenSelectedAddDiagnosticBackdrop"
	backdrop.color = Color(0.2,0.3,0.4,1.)
	backdrop.size = row_rect.size
	backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	backdrop.z_as_relative = true
	backdrop.z_index = -1
	owner.add_child(backdrop)
	overlay = owner.get_node_or_null("Parts/Part0") as TextureRect
	if not gate.require(overlay != null and overlay.texture != null,"existing decoded frozen selected texture required"): return false
	original_material = overlay.material
	var source := overlay.texture.get_image()
	if not gate.require(source != null and not source.is_empty(),"frozen selected atlas readable"): return false
	if not gate.require(source.save_png(shots.path_join(skin+"-frozen-selected-add-source.png")) == OK,"save frozen selected atlas source"): return false
	var state: Dictionary = client.trading_post_state()
	proof = {"product":product,"item_id":state.item_id,"name":state.selected_name,"row_rect":[row_rect.position.x,row_rect.position.y,row_rect.size.x,row_rect.size.y],"source_visible":has_visible_source(source),"source_size":[source.get_width(),source.get_height()]}
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
		current = control("PerksProgramFrozenSelected").get_node("Parts/Part0") as TextureRect
		saved = current.material
		current.material = reference_material
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if reference_material != null: current.material = saved
	gate.require(image.save_png(shots.path_join(skin+"-frozen-selected-add-"+label+".png")) == OK,"save frozen selected blend PNG")
	var crop := Rect2i(int(row_rect.position.x)+3,int(row_rect.position.y)+3,int(row_rect.size.x)-6,int(row_rect.size.y)-6)
	return image.get_region(crop)

func collect_selection_controls() -> bool:
	if not await click_checked("PerksProgramProduct%d" % ordinary): return false
	await motion(Vector2(1850,80))
	await frames(20)
	ordinary_after = selected_area("PerksProgramFrozenSelected") == 0.
	await capture_blend("ordinary-clears-frozen")
	if not await select_committed_frozen(): return false
	frozen_return = selected_area("PerksProgramFrozenSelected") > 0.
	await capture_blend("frozen-owner-return")
	return true

func collect_cart_controls() -> bool:
	if not await click_checked("PerksProgramProduct%d" % ordinary): return false
	if not await click_checked("PerksProgramAddToCartButton"): return false
	if not gate.require(client.trading_post_state().cart == PackedInt64Array([ordinary]),"one local ordinary cart member"): return false
	if not await select_committed_frozen(): return false
	if not await click_checked("PerksProgramViewCartButton"): return false
	await motion(Vector2(1850,80))
	await frames(20)
	var state: Dictionary = client.trading_post_state()
	cart_cleared_selection = bool(state.cart_visible) and int(state.selected) == -1 and selected_area("PerksProgramFrozenSelected") == 0.
	await capture_blend("cart-cleared")
	if not await click_checked("PerksProgramCartCloseButton"): return false
	await motion(Vector2(1850,80))
	await frames(20)
	state = client.trading_post_state()
	cart_restored_selection = not bool(state.cart_visible) and int(state.selected) == product and selected_area("PerksProgramFrozenSelected") > 0.
	await capture_blend("cart-return")
	if not await click_checked("PerksProgramProduct%d" % ordinary): return false
	if not await click_checked("PerksProgramRemoveFromCartButton"): return false
	if not gate.require(client.trading_post_state().cart.is_empty(),"local cart cleanup"): return false
	if not await select_committed_frozen(): return false
	await capture_blend("final-frozen-normal")
	return true

func write_frozen_receipt() -> bool:
	var file := FileAccess.open(receipts.path_join(skin+"-frozen-selected-add-result.json"),FileAccess.WRITE)
	if not gate.require(file != null,"write frozen selected ADD pixel receipt"): return false
	proof.ordinary_before = ordinary_before
	proof.ordinary_after = ordinary_after
	proof.frozen_return = frozen_return
	proof.cart_cleared_selection = cart_cleared_selection
	proof.cart_restored_selection = cart_restored_selection
	proof.pixel_tolerance = 1.5/255.
	proof.oracle = "existing committed frozen selected texture actual pixels versus same-atlas native ADD/MIX, pointer off slot, exact originals restored before verdict"
	file.store_string(JSON.stringify(proof,"\t"))
	file.close()
	return true
