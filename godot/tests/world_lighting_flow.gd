extends "res://tests/world_terrain_flow.gd"

# Requires GODOT_TEST_SERVER (private endpoint, never shared :5000),
# GODOT_TEST_ACCOUNT and GODOT_TEST_PASSWORD through world_units_flow.gd.
# That base fails before connecting if any input is unset and reuses all three
# for reconnect. Start the private server with GAME_SERVER_GROUND_DIR pointing
# to its ground bake; the account needs two characters (card 1 is selected).

func inspect_material_tiles(client: Node, parsed_tiles: Array) -> bool:
	if not super.inspect_material_tiles(client, parsed_tiles):
		return false
	var lighting := client.get_node_or_null("WorldLighting")
	if lighting == null:
		fail("Native world has no authored map/time lighting producer")
		return false
	var sun := lighting.get_node_or_null("Sun") as DirectionalLight3D
	var environment := lighting.get_node_or_null("Environment") as WorldEnvironment
	if sun == null or environment == null or environment.environment == null:
		fail("Native world lighting lacks directional sun/environment")
		return false
	if environment.environment.ambient_light_source != Environment.AMBIENT_SOURCE_DISABLED or environment.environment.reflected_light_source != Environment.REFLECTION_SOURCE_DISABLED:
		fail("Automatic lighting duplicates authored Retail lighting")
		return false
	var terrain := client.get_node("WorldTerrain")
	var material := terrain_chunks(terrain.get_child(0))[0].get_surface_override_material(0) as ShaderMaterial
	# Scene-lit materials read the scene light's global uniforms.
	var scene: Dictionary = client.account_state().scene_light
	var ambient: Vector3 = scene.ambient
	var direct: Vector3 = scene.direct
	var direction: Vector3 = scene.sun_direction
	var cube := material.get_shader_parameter("environment_map") as Cubemap
	if material.get_shader_parameter("scene_light") != true or ambient.is_equal_approx(Vector3.ONE) \
			or direct.length_squared() == 0.0 or cube == null or cube.get_width() != 32:
		fail("Native material retained fixture defaults instead of authored light/sky inputs")
		return false
	if not (-sun.global_basis.z).is_equal_approx(direction):
		fail("Native shadow sun is not aligned to Retail sun direction")
		return false
	var fog: Vector2 = scene.fog_range
	if fog.y <= fog.x or int(material.get_shader_parameter("fog_mode")) != 1:
		fail("Native material has no authored linear fog range")
		return false
	return true

