extends "res://tests/adventure_search_editing.gd"
# Observe from the selecting pointer event, not after the base fixture's settling/PNG work.
var flash_done := false
var flash_phase := 0
var flash_peak_alpha := 0.0
var flash_peak: Image
var flash_samples := []

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
	while Time.get_ticks_msec()-start < 5000:
		await process_frame
		var alpha := glow.modulate.a
		flash_samples.append([Time.get_ticks_msec()-start,alpha])
		# Discrete native frames need not land on the exact extrema. Pure tests cover
		# the authored continuous curve; here require two visible rises and a trough.
		if flash_phase == 0 and alpha > 0.55:
			flash_phase = 1
		elif flash_phase == 1 and alpha < 0.35:
			flash_phase = 2
		elif flash_phase == 2 and alpha > 0.55:
			flash_phase = 3
		if alpha > flash_peak_alpha:
			flash_peak_alpha = alpha
			await RenderingServer.frame_post_draw
			flash_peak = root.get_texture().get_image()
		if flash_phase == 3 and alpha < 0.01:
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
	if flash_phase != 3 or glow.modulate.a > 0.01 or flash_peak == null:
		stop("Source focus glow did not paint two pulses and return to zero; phase=" + str(flash_phase))
		return false
	await RenderingServer.frame_post_draw
	var settled := root.get_texture().get_image()
	var journal := control("EncounterJournal").get_global_rect()
	var effect := Rect2(header.get_global_rect().position+Vector2(-1,-1.5),Vector2(header.size.x+4,29)).grow(1)
	var inside := 0
	var outside := 0
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
	print("SECTION_FLASH_PIXELS inside=%s outside=%s peak=%s" % [inside,outside,flash_peak_alpha])
	if inside < 20 or outside != 0:
		stop("Focus flash pixel boundary differs inside=%s outside=%s" % [inside,outside])
		return false
	var prefix: String = client.account_state().ui_skin.to_lower()
	if flash_peak.save_png(shots.path_join(prefix+"-section-flash-peak.png")) != OK or settled.save_png(shots.path_join(prefix+"-section-flash-settled.png")) != OK:
		stop("Cannot persist focus flash frames")
		return false
	await create_timer(0.4).timeout
	if glow.modulate.a > 0.01:
		stop("Finished focus animation restarted without a new section focus")
		return false
	print("SECTION_FLASH_PASS skin=",client.account_state().ui_skin)
	return true
