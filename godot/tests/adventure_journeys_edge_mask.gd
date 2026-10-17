extends "res://tests/adventure_search_editing.gd"
# Source mask pixels against actual rendered art, with unmasked/background controls.

func frame_image() -> Image:
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()

func verify_texture_fade(name: String, faded: bool) -> bool:
	var owner := control(name)
	var part := owner.get_node("Parts/Part0") as TextureRect if owner != null else null
	if part == null or part.texture == null:
		stop("Missing source track art " + name)
		return false
	var clip := control("JourneyRenownTrackClip").get_global_rect()
	var bounds := part.get_global_rect().intersection(clip)
	var reference := Image.load_from_file(OS.get_environment("ADVENTURE_MASK_REFERENCE"))
	if reference == null or reference.get_size() != Vector2i(1024,128):
		stop("Authenticated source1024x128 alpha reference required")
		return false
	# Freeze the client update while sampling one frame's art/background controls;
	# normal projection must not overwrite those temporary measurement controls.
	client.set_process(false)
	var painted := await frame_image()
	var material := part.material
	part.material = null
	await settle()
	var plain := await frame_image()
	part.visible = false
	await settle()
	var background := await frame_image()
	part.visible = true
	part.material = material
	await settle()
	client.set_process(true)
	var samples := 0
	var worst := 0.0
	for y in range(ceili(bounds.position.y), floori(bounds.end.y)):
		for x in range(ceili(bounds.position.x), floori(bounds.end.x)):
			var relative := Vector2(x+0.5,y+0.5)-clip.get_center()+Vector2(512,64)
			var alpha := reference.get_pixel(clampi(int(relative.x),0,1023),clampi(int(relative.y),0,127)).r
			if (faded and alpha > 0.25) or (not faded and alpha < 0.99):
				continue
			var base := background.get_pixel(x,y)
			var raw := plain.get_pixel(x,y)
			var current := painted.get_pixel(x,y)
			var delta := Vector3(raw.r-base.r,raw.g-base.g,raw.b-base.b).length()
			if delta < 0.15:
				continue
			var actual := Vector3(current.r-base.r,current.g-base.g,current.b-base.b).length()/delta
			samples += 1
			worst = maxf(worst,actual) if faded else maxf(worst,absf(actual-1.0))
	if samples < 8 or worst > (0.7 if faded else 0.12):
		await shot("journeys-edge-mask-red")
		stop("Source texture fade differs %s faded=%s samples=%s worst=%s" % [name,faded,samples,worst])
		return false
	print("JOURNEYS_EDGE_TEXTURE name=%s faded=%s samples=%s worst=%s" % [name,faded,samples,worst])
	return true

func verify_opaque_caption() -> bool:
	var label := control("JourneyRenownLevel1Level") as Label
	if label == null or label.text != "1":
		stop("Real edge-level caption absent")
		return false
	var bounds := label.get_global_rect()
	client.set_process(false)
	var painted := await frame_image()
	var copy := label.duplicate() as Label
	# RegistryUi is a CanvasLayer; a viewport-root copy draws behind the journal.
	# Keep that draw layer while removing the track's ancestors from the control.
	var canvas: Node = label.get_parent()
	while canvas != null and not canvas is CanvasLayer:
		canvas = canvas.get_parent()
	if canvas == null:
		stop("Caption has no native canvas layer")
		return false
	canvas.add_child(copy)
	copy.position = bounds.position
	copy.size = bounds.size
	copy.modulate = Color.WHITE
	copy.self_modulate = Color.WHITE
	copy.z_as_relative = false
	copy.z_index = label.z_index
	label.visible = false
	await settle()
	var independent := await frame_image()
	copy.queue_free()
	label.visible = true
	await settle()
	client.set_process(true)
	var differences := 0
	for y in range(ceili(bounds.position.y),floori(bounds.end.y)):
		for x in range(ceili(bounds.position.x),floori(bounds.end.x)):
			var a := painted.get_pixel(x,y)
			var b := independent.get_pixel(x,y)
			if Vector3(a.r-b.r,a.g-b.g,a.b-b.b).length() > 0.04:
				differences += 1
	if differences > 4:
		stop("Level FontString differs from opaque same-canvas reference; changed pixels=" + str(differences))
		return false
	print("JOURNEYS_EDGE_CAPTION opaque1 differences=", differences)
	return true

func verify_section_selection() -> bool:
	if not await super.verify_section_selection():
		return false
	var deadline := Time.get_ticks_msec()+90000
	while client.adventure_guide_state().renown_received_faction_count < 0 and Time.get_ticks_msec()<deadline:
		await process_frame
	if not await click("EncounterJournalJourneysTab") or not await click("JournalJourneysExpansion") or not await click("JournalJourneysExpansion9") or not await click("JourneyCard12"):
		return false
	if not await click("JourneyRenownLevel5"):
		return false
	if not await verify_texture_fade("JourneyRenownLevel1Border",true) or not await verify_texture_fade("JourneyRenownLevel5Border",false) or not await verify_opaque_caption():
		return false
	if not await shot("journeys-edge-mask-level5"):
		return false
	# Move source art under the stationary mask; uniforms must not retain old positions.
	if not await wheel("JourneyRenownTrackClip",MOUSE_BUTTON_WHEEL_DOWN):
		return false
	if not await verify_texture_fade("JourneyRenownLevel2Border",true) or not await verify_texture_fade("JourneyRenownLevel6Border",false):
		return false
	if not await shot("journeys-edge-mask-level6"):
		return false
	print("JOURNEYS_EDGE_MASK_PASS skin=",client.account_state().ui_skin)
	return true
