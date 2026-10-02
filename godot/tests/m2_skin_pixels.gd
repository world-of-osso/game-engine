extends "res://tests/m2_loader_pixels.gd"

const THIRD := Color(0.8, 0.2, 0.6)
const STATIC := Color(0.1, 0.8, 0.35)
const SKIN_FDID := 910099

var authored_type := 11
var second_type := 12
var overrides := PackedInt64Array([910001, 910002, 910003])

func make_m2(flags: int, blend_mode: int) -> PackedByteArray:
	var original := super.make_m2(flags, blend_mode)
	var md20 := original.slice(8, 8 + original.decode_u32(4))
	md20.resize(0x270)
	put_u32(md20, 0x54, 0x240) # Two authored texture records.
	put_u32(md20, 0x240, authored_type)
	put_u32(md20, 0x250, second_type)
	var txid := PackedByteArray()
	txid.resize(8)
	put_u32(txid, 0, 910004) # Static type-0 texture, never a creature override.
	put_u32(txid, 4, 910004)
	var sfid := PackedByteArray()
	sfid.resize(4)
	put_u32(sfid, 0, SKIN_FDID) # Geometry ID is not a texture source.
	var model := chunk("MD21", md20)
	model.append_array(chunk("TXID", txid))
	model.append_array(chunk("SFID", sfid))
	return model

func prepare_fixture(flags: int, shader_id: int, texture_count: int, blend_mode: int = 0) -> bool:
	return super.prepare_fixture(flags, shader_id, texture_count, blend_mode) \
		and write_fixture(FIXTURE + "/textures/910003.blp", make_blp(THIRD)) \
		and write_fixture(FIXTURE + "/textures/910004.blp", make_blp(STATIC))

func load_quad() -> bool:
	if loaded != null:
		loaded.queue_free()
		await process_frame
	if not loader.has_method("load_m2_with_skin_fdids"):
		push_error("RED: WowAssetLoader.load_m2_with_skin_fdids is unavailable")
		return false
	var result: Dictionary = loader.call("load_m2_with_skin_fdids", FIXTURE + "/models/quad.m2", overrides)
	if result.has("error") or not result.get("node") is Node3D:
		push_error("M2 skin load: " + str(result.get("error", "no node")))
		return false
	if not result.get("missing_texture_fdids", PackedInt32Array()).is_empty():
		push_error("Unexpected missing texture FDIDs: " + str(result.missing_texture_fdids))
		result.node.free()
		return false
	loaded = result.node
	viewport.add_child(loaded)
	return true

func cleanup() -> void:
	for filename in ["910003.blp", "910004.blp"]:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(FIXTURE + "/textures/" + filename))
	super.cleanup()

func run_pixels() -> bool:
	for test_case in [
		[11, BASE, "creature slot 0 type 11"],
		[12, SECOND, "creature slot 1 type 12"],
		[13, THIRD, "creature slot 2 type 13"],
		[2, BASE, "creature slot 0 type 2"],
		[0, STATIC, "static type 0 ignores creature overrides"],
	]:
		authored_type = test_case[0]
		if not prepare_fixture(7, 0x10, 1) or not await load_quad():
			return false
		if not await assert_pixel(test_case[2], test_case[1]):
			return false
	authored_type = 11
	second_type = 12
	if not prepare_fixture(7, 0x10, 2, 2) or not await load_quad():
		return false
	var combined := Color(BASE.r * SECOND.r, BASE.g * SECOND.g, BASE.b * SECOND.b)
	if not await assert_pixel("effect uses creature slots 0 and 1", combined):
		return false
	overrides = PackedInt64Array([0, 0, 0])
	if not prepare_fixture(7, 0x10, 1) or not await load_quad():
		return false
	# An unbound Godot sampler reads white; the loader must not synthesize a texture.
	var material := loaded.get_node("Batch0").get_active_material(0) as ShaderMaterial
	if material.get_shader_parameter("base_texture") != null:
		push_error("Zero creature slot unexpectedly bound a texture")
		return false
	if not await assert_pixel("zero creature slot leaves sampler unbound", Color.WHITE):
		return false
	return assert_invalid_slots()

func assert_invalid_slots() -> bool:
	for slots in [PackedInt64Array([1, 2]), PackedInt64Array([-1, 0, 0]), PackedInt64Array([4294967296, 0, 0])]:
		var result: Dictionary = loader.call("load_m2_with_skin_fdids", FIXTURE + "/models/quad.m2", slots)
		if not result.has("error") or result.has("node"):
			push_error("Invalid skin texture slots were accepted: " + str(slots))
			if result.get("node") is Node3D:
				result.node.free()
			return false
	print("PASS: invalid creature texture slots rejected")
	return true
