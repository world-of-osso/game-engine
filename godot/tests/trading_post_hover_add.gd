extends "res://tests/trading_post_cart_selection.gd"
# Opt-in source ADD pixel oracle only; every counterfactual is restored before verdict.
var overlay: TextureRect
var original_material: Material
var backdrop: ColorRect
var row_rect := Rect2()
var product := 0
var proof: Dictionary = {}

func run_cases() -> bool:
	if not gate.require(OS.get_environment("TP_HOVER_ADD_OPT_IN") == "1","explicit hover ADD pixel proof opt-in required"): return false
	if not gate.step(await connect_private(),"private account/world/vendor"): return false
	baseline = client.trading_post_state().duplicate(true)
	appearances = Array(client.tooltip_state().appearance_collection)
	appearances.sort()
	origin = client.account_state().local_player_position
	if not gate.require(baseline.cart.is_empty() and int(baseline.frozen) > 0,"empty cart/genuine reservation required"): return false
	if not await only_type(1): return false
	for entry in client.trading_post_state().rows:
		var fields := str(entry).split("|")
		if fields[1] == "1" and int(fields[0]) != int(client.trading_post_state().selected):
			product = int(fields[0])
			break
	if not gate.require(product > 0,"observed unselected ordinary product required"): return false
	if not await collect_hover_pixels():
		restore_counterfactual()
		return false
	if not await restore_original_state(): return false
	if not write_blend_receipt(): return false
	# Assertions follow restoration even for the expected RED.
	var cases := [
		[bool(proof.source_visible),"decoded authored overlay contains visible source texels"],
		[bool(proof.backdrop_stable),"opaque diagnostic backdrop stays stable"],
		[int(proof.actual_add_different_pixels) == 0,"actual hover pixels must equal source ADD reference, differing pixels=%d" % int(proof.actual_add_different_pixels)],
		[int(proof.add_alpha_different_pixels) >= 32,"authored pixels genuinely distinguish ADD from alpha"],
		[int(proof.restored_actual_different_pixels) == 0,"original material restores actual pixels exactly"],
		[bool(proof.leave_overlay_absent),"pointer leave removes the existing hover overlay"],
		[bool(proof.authority_restored),"authority/local/HUD/world conserved without transactions"]
	]
	for result in cases:
		if not gate.require(bool(result[0]),str(result[1])): return false
		if not gate.case_done(): return false
	return true

func collect_hover_pixels() -> bool:
	await motion(Vector2(1850,80))
	await frames(20)
	var row := control("PerksProgramProduct%d" % product)
	if not gate.require(row != null,"native ordinary hover owner"): return false
	row_rect = row.get_global_rect()
	backdrop = ColorRect.new()
	backdrop.name = "HoverAddDiagnosticBackdrop"
	backdrop.color = Color(0.2,0.3,0.4,1.)
	backdrop.size = row_rect.size
	backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	backdrop.z_as_relative = false
	backdrop.z_index = 3999
	# Share the row's canvas layer; a viewport-root Control would remain behind its UI.
	row.get_parent().add_child(backdrop)
	backdrop.global_position = row_rect.position
	var off := await capture_blend("backdrop-off")
	await motion(row_rect.get_center())
	await frames(20)
	var parts := row.get_node_or_null("Parts")
	if not gate.require(parts != null and parts.get_child_count() == 1,"one existing ordinary hover part, no synthetic overlay"): return false
	overlay = parts.get_child(0) as TextureRect
	if not gate.require(overlay != null and overlay.texture != null,"decoded native hover texture required"): return false
	original_material = overlay.material
	var source := overlay.texture.get_image()
	if not gate.require(source != null and not source.is_empty(),"source atlas image readable"): return false
	var source_path := shots.path_join(skin+"-hover-add-source.png")
	if not gate.require(source.save_png(source_path) == OK,"preserve decoded source atlas image"): return false
	proof.source_visible = has_visible_source(source)
	proof.source_size = [source.get_width(),source.get_height()]
	proof.row_rect = [row_rect.position.x,row_rect.position.y,row_rect.size.x,row_rect.size.y]
	proof.source_texture_class = overlay.texture.get_class()
	proof.source_texture_region = str(overlay.texture.region) if overlay.texture is AtlasTexture else "full"
	var actual := await capture_blend("actual-hover")
	var add := CanvasItemMaterial.new()
	add.blend_mode = CanvasItemMaterial.BLEND_MODE_ADD
	overlay.material = add
	var reference := await capture_blend("reference-add")
	var alpha := CanvasItemMaterial.new()
	alpha.blend_mode = CanvasItemMaterial.BLEND_MODE_MIX
	overlay.material = alpha
	var alpha_image := await capture_blend("reference-alpha")
	overlay.material = original_material
	var restored := await capture_blend("actual-restored")
	proof.actual_add_different_pixels = count_pixel_differences(actual,reference)
	proof.add_alpha_different_pixels = count_pixel_differences(reference,alpha_image)
	proof.actual_alpha_different_pixels = count_pixel_differences(actual,alpha_image)
	proof.restored_actual_different_pixels = count_pixel_differences(actual,restored)
	await motion(Vector2(1850,80))
	await frames(20)
	var off_again := await capture_blend("backdrop-leave")
	proof.backdrop_stable = count_pixel_differences(off,off_again) == 0
	proof.leave_overlay_absent = parts.get_child_count() == 0
	restore_counterfactual()
	await frames(20)
	await capture_blend("normal-restored")
	return true

func has_visible_source(image: Image) -> bool:
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var pixel := image.get_pixel(x,y)
			if pixel.a > 0.1 and maxf(pixel.r,maxf(pixel.g,pixel.b)) > 0.1: return true
	return false

func count_pixel_differences(a: Image, b: Image) -> int:
	var count := 0
	for y in range(a.get_height()):
		for x in range(a.get_width()):
			var p := a.get_pixel(x,y)
			var q := b.get_pixel(x,y)
			if maxf(absf(p.r-q.r),maxf(absf(p.g-q.g),absf(p.b-q.b))) > 1.5/255.: count += 1
	return count

func capture_blend(label: String) -> Image:
	await frames(4)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := shots.path_join(skin+"-hover-add-"+label+".png")
	gate.require(image.save_png(path) == OK,"save native hover blend PNG")
	# Inset avoids viewport fractional edges; the backdrop blocks animated world/text below.
	var crop := Rect2i(int(row_rect.position.x)+3,int(row_rect.position.y)+3,int(row_rect.size.x)-6,int(row_rect.size.y)-6)
	return image.get_region(crop)

func restore_counterfactual() -> void:
	if is_instance_valid(overlay): overlay.material = original_material
	if is_instance_valid(backdrop):
		backdrop.get_parent().remove_child(backdrop)
		backdrop.queue_free()

func restore_original_state() -> bool:
	restore_counterfactual()
	if not await only_type(0): return false
	if not await click_checked("PerksProgramProduct%d" % int(baseline.selected)): return false
	var state: Dictionary = client.trading_post_state()
	var final_appearances := Array(client.tooltip_state().appearance_collection)
	final_appearances.sort()
	var same := int(state.tender) == int(baseline.tender) and int(state.frozen) == int(baseline.frozen) and state.cart == baseline.cart and int(state.selected) == int(baseline.selected) and int(state.items) == int(baseline.items) and final_appearances == appearances
	var delta: float = client.account_state().local_player_position.distance_to(origin)
	var chat := client.get_node_or_null("ChatFrameUI") as CanvasLayer
	if not gate.require(chat != null and not chat.visible,"fullscreen HUD owner before Leave"): return false
	if not await click_checked("PerksProgramLeaveButton"): return false
	if not await until(func(): return not client.trading_post_state().open and chat.visible,"Leave restores HUD"): return false
	proof.authority_restored = same and delta < 0.0001
	proof.world_delta = delta
	proof.final = state
	proof.baseline = baseline
	proof.appearances = appearances
	proof.skin = skin
	proof.pixel_tolerance = 1.5/255.
	proof.oracle = "actual same-atlas pixels compared with forced native CanvasItemMaterial ADD reference; forced MIX distinguishes blend; restore original material before verdict"
	return true

func write_blend_receipt() -> bool:
	var file := FileAccess.open(receipts.path_join(skin+"-hover-add-result.json"),FileAccess.WRITE)
	if not gate.require(file != null,"write hover ADD pixel receipt"): return false
	file.store_string(JSON.stringify(proof,"\t"))
	file.close()
	return true
