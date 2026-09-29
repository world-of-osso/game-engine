extends "res://tests/m2_loader_pixels.gd"

# A DXT1 BLP with 1-bit alpha (Elwynn bush 189700's leaf texture 189937 is one) on an
# alpha-key (blend 1) batch over a known background. A punch-through texel must show
# the background: WebWowViewerCpp uploads such BLPs as BC1 RGBA (BlpTexture.cpp
# getTextureType, GBlpTextureVLK.cpp), where it decodes transparent and the 224/255
# alpha test discards it. Godot's FORMAT_DXT1 is BC1 RGB, where it decodes opaque black.
const BACKGROUND := Color(0.2, 0.6, 0.1)
# RGB565 (16, 32, 8) expands exactly to 8-bit (132, 130, 66).
const TEXEL_565 := (16 << 11) | (32 << 5) | 8
const TEXEL := Color(132.0 / 255.0, 130.0 / 255.0, 66.0 / 255.0)
# Every texel's 2-bit index: 0 is colour 0, 3 is transparent while colour 0 <= colour 1.
const OPAQUE_INDICES := 0x00000000
const TRANSPARENT_INDICES := 0xFFFFFFFF

func make_dxt1_blp(indices: int) -> PackedByteArray:
	# BLP2 DXT1 with alpha depth 1: 148-byte header, 1024-byte palette, one 4x4 block.
	var bytes := PackedByteArray()
	bytes.resize(148 + 1024 + 8)
	put_magic(bytes, 0, "BLP2")
	put_u32(bytes, 4, 1) # Direct content.
	bytes[8] = 2 # DXT compression.
	bytes[9] = 1 # Alpha depth.
	bytes[10] = 0 # DXT1.
	put_u32(bytes, 12, 4)
	put_u32(bytes, 16, 4)
	put_u32(bytes, 20, 1172)
	put_u32(bytes, 84, 8)
	# Equal colours select the three-colour mode that has the punch-through index.
	put_u16(bytes, 1172, TEXEL_565)
	put_u16(bytes, 1174, TEXEL_565)
	put_u32(bytes, 1176, indices)
	return bytes

# Loaded textures are shared per FDID, so each case names its own.
var texture_fdid := 910003

func make_m2(flags: int, blend_mode: int) -> PackedByteArray:
	var model := super.make_m2(flags, blend_mode)
	put_u32(model, model.size() - 8, texture_fdid) # TXID entry 0.
	return model

func prepare_dxt1(fdid: int, indices: int) -> bool:
	texture_fdid = fdid
	return prepare_fixture(7, 0x10, 1, 1) \
		and write_fixture(FIXTURE + "/textures/%d.blp" % fdid, make_dxt1_blp(indices))

func add_background() -> void:
	var background := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(4.0, 4.0)
	background.mesh = plane
	background.position = Vector3(0.0, -0.5, 0.0)
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = BACKGROUND
	background.material_override = material
	viewport.add_child(background)

func run_pixels() -> bool:
	add_background()
	if not prepare_dxt1(910003, OPAQUE_INDICES) or not await load_quad():
		return false
	var opaque_passed := await assert_pixel("1-bit-alpha DXT1 opaque texel", TEXEL)
	if not prepare_dxt1(910004, TRANSPARENT_INDICES) or not await load_quad():
		return false
	var transparent_passed := await assert_pixel("1-bit-alpha DXT1 punch-through texel", BACKGROUND)
	for fdid in [910003, 910004]:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(FIXTURE + "/textures/%d.blp" % fdid))
	return opaque_passed and transparent_passed
