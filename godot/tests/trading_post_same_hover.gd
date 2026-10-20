extends "res://tests/trading_post_hover_add.gd"
# Step1 only: observe native input order and painted label ink; never mutate colours.
const A := 642
const B := 848
var samples: Dictionary = {}
var pointer_anchor := Vector2.ZERO
var initial_scroll := Vector2.ZERO
var initial_a_rect := Rect2()
var down_kept_hover_and_geometry := false
var selection_back_kept_hover := false

func _initialize() -> void:
	Engine.max_fps = 60
	gate = Completion.new(self, 9)
	call_deferred("run_fixture")

func run_cases() -> bool:
	if not gate.require(OS.get_environment("TP_SAME_HOVER_OPT_IN") == "1","explicit same-hover diagnostic opt-in required"): return false
	if not gate.step(await connect_private(),"private account/world/vendor"): return false
	baseline = client.trading_post_state().duplicate(true)
	appearances = Array(client.tooltip_state().appearance_collection)
	appearances.sort()
	origin = client.account_state().local_player_position
	if not gate.require(baseline.cart.is_empty() and int(baseline.frozen) > 0,"empty cart/genuine reservation required"): return false
	var collected := await collect_ordered_inputs()
	# Restore original authority/local/HUD even when a collected colour is the expected RED.
	if not await restore_original_state(): return false
	if not write_order_receipt(): return false
	if not collected: return false
	var cases := [
		[down_kept_hover_and_geometry,"native Down selects B with pointer still on A and no scroll/reposition"],
		[has_colour("selected-a-hover","a",true) and has_colour("selected-a-hover","b",false),"initial selected-hover A WHITE and unselected B gold are painted"],
		[has_colour("deselected-a-same-hover","a",false),"same-hover deselected A must retain Retail SetSelected(false) NORMAL gold"],
		[has_colour("deselected-a-same-hover","b",true),"keyboard-selected B is painted WHITE"],
		[has_colour("deselected-a-leave","a",false),"real leave keeps unselected A painted gold"],
		[has_colour("deselected-a-reenter","a",true),"real re-enter makes unselected A painted WHITE"],
		[selection_back_kept_hover and has_colour("reselected-a-same-hover","a",true) and has_colour("reselected-a-same-hover","b",false),"native Up reselects A without mouse motion; A WHITE/B gold"],
		[has_colour("selected-a-leave","a",true),"selected A stays painted WHITE on real leave"],
		[bool(proof.authority_restored),"exact authority/local/HUD/world conserved without transactions"]
	]
	for result in cases:
		if not gate.require(bool(result[0]),str(result[1])): return false
		if not gate.case_done(): return false
	return true

func collect_ordered_inputs() -> bool:
	if not await only_type(1): return false
	var rows: PackedStringArray = client.trading_post_state().rows
	var index_a := rows.find("%d|1" % A)
	if not gate.require(index_a >= 0 and index_a+1 < rows.size() and rows[index_a+1] == "%d|1" % B,"observed available adjacent A642/B848 required"): return false
	if not await click_checked("PerksProgramProduct%d" % A): return false
	await frames(20)
	var state: Dictionary = client.trading_post_state()
	if not gate.require(int(state.selected) == A and str(state.selected_name) == "Lavish Floral Stalk","real metadata-bearing selected A required"): return false
	var row_a := control("PerksProgramProduct%d" % A)
	var row_b := control("PerksProgramProduct%d" % B)
	var list := control("PerksProgramScrollBox")
	var scroll := control("PerksProgramScrollChild")
	if not gate.require(row_a != null and row_b != null and list != null and scroll != null,"native product/list controls required"): return false
	initial_a_rect = row_a.get_global_rect()
	if not gate.require(list.get_global_rect().encloses(initial_a_rect) and list.get_global_rect().encloses(row_b.get_global_rect()),"both adjacent products fully visible before Down"): return false
	# Label-side point, not the trailing child cart-toggle owner.
	pointer_anchor = initial_a_rect.position + Vector2(150.,initial_a_rect.size.y/2.)
	await motion(pointer_anchor)
	await frames(20)
	initial_scroll = scroll.position
	await record_order_sample("selected-a-hover")
	await tap(KEY_DOWN)
	await frames(20)
	state = client.trading_post_state()
	down_kept_hover_and_geometry = int(state.selected) == B and same_hover_geometry()
	await record_order_sample("deselected-a-same-hover")
	await motion(Vector2(1850,80))
	await frames(20)
	await record_order_sample("deselected-a-leave")
	await motion(pointer_anchor)
	await frames(20)
	await record_order_sample("deselected-a-reenter")
	await tap(KEY_UP)
	await frames(20)
	state = client.trading_post_state()
	selection_back_kept_hover = int(state.selected) == A and same_hover_geometry()
	await record_order_sample("reselected-a-same-hover")
	await motion(Vector2(1850,80))
	await frames(20)
	await record_order_sample("selected-a-leave")
	return true

func same_hover_geometry() -> bool:
	var row := control("PerksProgramProduct%d" % A)
	var scroll := control("PerksProgramScrollChild")
	if row == null or scroll == null: return false
	var parts := row.get_node_or_null("Parts")
	return root.get_mouse_position().distance_to(pointer_anchor) < 0.01 and row.get_global_rect() == initial_a_rect and scroll.position == initial_scroll and parts != null and parts.get_child_count() == 1

func record_order_sample(label: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	gate.require(image.save_png(shots.path_join(skin+"-same-hover-"+label+".png")) == OK,"save native same-hover frame")
	var result := {"state":client.trading_post_state().duplicate(true),"pointer":[root.get_mouse_position().x,root.get_mouse_position().y],"scroll":[control("PerksProgramScrollChild").position.x,control("PerksProgramScrollChild").position.y]}
	for entry in [["a",A],["b",B]]:
		var name := control("PerksProgramName%d" % int(entry[1]))
		var rgba: Color = name.get_theme_color("font_color")
		var rect := name.get_global_rect()
		var crop := image.get_region(Rect2i(int(rect.position.x),int(rect.position.y),int(rect.size.x),int(rect.size.y)))
		var key := str(entry[0])
		gate.require(crop.save_png(shots.path_join(skin+"-same-hover-"+label+"-"+key+"-glyph.png")) == OK,"save actual label glyph crop")
		result[key] = {"rgba":[rgba.r,rgba.g,rgba.b,rgba.a],"rect":[rect.position.x,rect.position.y,rect.size.x,rect.size.y],"white_ink_pixels":count_ink(crop,Color.WHITE),"gold_ink_pixels":count_ink(crop,Color(1.,210./255.,0.,1.))}
	samples[label] = result

func count_ink(image: Image, expected: Color) -> int:
	var count := 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			var p := image.get_pixel(x,y)
			if maxf(absf(p.r-expected.r),maxf(absf(p.g-expected.g),absf(p.b-expected.b))) < 0.04: count += 1
	return count

func has_colour(label: String, owner: String, white: bool) -> bool:
	var entry: Dictionary = samples[label][owner]
	var expected := [1.,1.,1.,1.] if white else [1.,210./255.,0.,1.]
	for channel in range(4):
		if absf(float(entry.rgba[channel])-float(expected[channel])) > 0.000001: return false
	return int(entry.white_ink_pixels if white else entry.gold_ink_pixels) >= 8

func write_order_receipt() -> bool:
	var file := FileAccess.open(receipts.path_join(skin+"-same-hover-result.json"),FileAccess.WRITE)
	if not gate.require(file != null,"write same-hover diagnostic receipt"): return false
	proof.samples = samples
	proof.down_kept_hover_and_geometry = down_kept_hover_and_geometry
	proof.selection_back_kept_hover = selection_back_kept_hover
	proof.oracle = "native fixed-pointer A642 Down to visible B848; actual LabelRGBA plus actual screenshot glyph ink; RetailSetSelected(false)gold expectation; no injected state or colour mutation"
	proof.rgba_tolerance = 0.000001
	proof.glyph_rgb_tolerance = 0.04
	proof.glyph_minimum_pixels = 8
	file.store_string(JSON.stringify(proof,"\t"))
	file.close()
	return true
