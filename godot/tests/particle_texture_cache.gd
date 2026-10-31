extends "res://tests/particle_blend_pixels.gd"

var control_texture: Texture2D

func control_sync() -> void:
	RenderingServer.texture_get_rd_texture(control_texture.get_rid())

func solid_image(color: Color) -> Image:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return image

func run_cases() -> void:
	probe = ClassDB.instantiate("WowParticleProbe")
	if probe == null or not prepare_fixture(7, 0x10, 1):
		quit(1)
		return
	make_viewport()
	add_background()
	if not await show_quad(0, Color.WHITE, 1.0):
		quit(1)
		return
	var material := (particle as GeometryInstance3D).material_override as ShaderMaterial
	var texture := ImageTexture.create_from_image(solid_image(Color.RED))
	material.set_shader_parameter("texture_0", texture)
	var passed := await assert_pixel("initial particle texture", Color.RED)
	# Positive control: prove that this Godot build reports repeated texture syncs.
	control_texture = texture
	print("PARTICLE_TEXTURE_CONTROL_BEGIN")
	RenderingServer.frame_pre_draw.connect(control_sync)
	for frame in 60:
		await process_frame
	RenderingServer.frame_pre_draw.disconnect(control_sync)
	print("PARTICLE_TEXTURE_CONTROL_END")
	for frame in 10:
		await process_frame
		await RenderingServer.frame_post_draw
	# Bracket only settled frames: the runner checks that the render server reports no
	# texture_get_rd_texture synchronization during this interval.
	print("PARTICLE_TEXTURE_STEADY_BEGIN")
	for frame in 120:
		await process_frame
		await RenderingServer.frame_post_draw
	print("PARTICLE_TEXTURE_STEADY_END")
	texture.set_image(solid_image(Color.GREEN))
	passed = await assert_pixel("changed particle texture resource", Color.GREEN) and passed
	material.set_shader_parameter("texture_0", ImageTexture.create_from_image(solid_image(Color.BLUE)))
	passed = await assert_pixel("replaced particle texture resource", Color.BLUE) and passed
	particle.free()
	cleanup()
	probe = null
	quit(0 if passed else 1)
