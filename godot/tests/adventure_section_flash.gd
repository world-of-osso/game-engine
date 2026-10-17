extends "res://tests/adventure_search_editing.gd"
# Observe from the selecting pointer event, not after the base fixture's settling/PNG work.
var flash_done := false
var flash_phase := 0
var flash_peak_alpha := 0.0
var flash_peak: Image
var flash_samples := []
var flash_peak_owners := []

func visible_paint_owners() -> Array:
	var owners := []
	var pending: Array[Node] = [client]
	var interior := control("EncounterJournal").get_global_rect().grow(-12)
	while not pending.is_empty():
		var node: Node = pending.pop_back()
		for child in node.get_children():
			pending.append(child)
		var paint := node as Control
		if paint == null or not paint.is_visible_in_tree() or not interior.intersects(paint.get_global_rect()):
			continue
		if not paint is Label and not paint is TextureRect:
			continue
		var rect := paint.get_global_rect()
		var entry := {"path":str(paint.get_path()),"class":paint.get_class(),"rect":[rect.position.x,rect.position.y,rect.size.x,rect.size.y],"alpha":paint.modulate.a,"z":paint.z_index}
		if paint is TextureRect:
			entry["texture_ready"] = paint.texture != null
		if paint is Label:
			entry["text"] = paint.text
		owners.append(entry)
	return owners

func click(name: String) -> bool:
	if name == "JournalSearchPreview0" and label_text("EncounterJournalSearchBox") == "Paralyze":
		call_deferred("observe_focus_flash")
	return await super.click(name)

func observe_focus_flash() -> void:
	var deadline := Time.get_ticks_msec()+8000
	while control("JournalSection2410Glow") == null and Time.get_ticks_msec()<deadline:
		await process_frame
	var glow := control("JournalSection2410Glow")
	if glow == null:
		flash_done = true
		return
	var start := Time.get_ticks_msec()
	var previous_alpha := 0.0
	while Time.get_ticks_msec()-start < 5000:
		await process_frame
		var alpha := glow.modulate.a
		flash_samples.append([Time.get_ticks_msec()-start,alpha])
		# Detect direction changes, not a sampled near-zero trough: a rendered
		# frame can span the trough without changing the source's continuous curve.
		var change := alpha-previous_alpha
		if flash_phase == 0 and change > 0.0001 and alpha > 0.2:
			flash_phase = 1
		elif flash_phase == 1 and change < -0.0001 and previous_alpha > 0.5:
			flash_phase = 2
		elif flash_phase == 2 and change > 0.0001 and alpha > 0.2:
			flash_phase = 3
		elif flash_phase == 3 and change < -0.0001 and previous_alpha > 0.5:
			flash_phase = 4
		# One bright-frame readback avoids stalling the observer at every new max.
		if flash_peak == null and alpha > 0.75:
			flash_peak_alpha = alpha
			await RenderingServer.frame_post_draw
			flash_peak = root.get_texture().get_image()
			flash_peak_owners = visible_paint_owners()
		previous_alpha = alpha
		if flash_phase == 4 and alpha < 0.01:
			break
	flash_done = true

func verify_section_selection() -> bool:
	if not await super.verify_section_selection():
		return false
	var header := control("JournalSection2410")
	var glow := control("JournalSection2410Glow")
	if glow == null:
		await shot("section-flash-missing-red")
		stop("Source EncounterInfoTemplate focus glow missing after physical Paralyze selection")
		return false
	while not flash_done:
		await process_frame
	print("SECTION_FLASH_SAMPLES ",JSON.stringify(flash_samples))
	if flash_phase != 4 or glow.modulate.a > 0.01 or flash_peak == null:
		stop("Source focus glow did not paint two pulses and return to zero; phase=" + str(flash_phase))
		return false
	await RenderingServer.frame_post_draw
	var settled := root.get_texture().get_image()
	var settled_owners := visible_paint_owners()
	var journal := control("EncounterJournal").get_global_rect()
	var effect := Rect2(header.get_global_rect().position+Vector2(-1,-1.5),Vector2(header.size.x+4,29)).grow(1)
	var inside := 0
	var outside := 0
	var outside_min := Vector2i(100000,100000)
	var outside_max := Vector2i.ZERO
	var interior := journal.grow(-12)
	for y in range(ceili(interior.position.y),floori(interior.end.y)):
		for x in range(ceili(interior.position.x),floori(interior.end.x)):
			var a := flash_peak.get_pixel(x,y)
			var b := settled.get_pixel(x,y)
			if Vector3(a.r-b.r,a.g-b.g,a.b-b.b).length() <= 0.04:
				continue
			if effect.has_point(Vector2(x,y)):
				inside += 1
			else:
				outside += 1
				outside_min.x = mini(outside_min.x,x)
				outside_min.y = mini(outside_min.y,y)
				outside_max.x = maxi(outside_max.x,x)
				outside_max.y = maxi(outside_max.y,y)
	print("SECTION_FLASH_PIXELS inside=%s outside=%s peak=%s effect=%s outside_min=%s outside_max=%s" % [inside,outside,flash_peak_alpha,effect,outside_min,outside_max])
	var prefix: String = client.account_state().ui_skin.to_lower()
	var owner_path := "/home/osso/.worktrees/adventure-guide6-proof/"+shots.get_file()+"-"+prefix+"-paint-owners.json"
	var owner_file := FileAccess.open(owner_path,FileAccess.WRITE)
	if owner_file == null:
		stop("Cannot persist paint-owner diagnostic")
		return false
	owner_file.store_string(JSON.stringify({"peak":flash_peak_owners,"settled":settled_owners,"outside_min":[outside_min.x,outside_min.y],"outside_max":[outside_max.x,outside_max.y],"outside_count":outside}))
	owner_file.close()
	if flash_peak.save_png(shots.path_join(prefix+"-section-flash-peak.png")) != OK or settled.save_png(shots.path_join(prefix+"-section-flash-settled.png")) != OK:
		stop("Cannot persist focus flash frames")
		return false
	if inside < 20 or outside != 0:
		stop("Focus flash pixel boundary differs inside=%s outside=%s" % [inside,outside])
		return false
	await create_timer(0.4).timeout
	if glow.modulate.a > 0.01:
		stop("Finished focus animation restarted without a new section focus")
		return false
	print("SECTION_FLASH_PASS skin=",client.account_state().ui_skin)
	return true
