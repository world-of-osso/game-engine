extends SceneTree

const Appearance := preload("res://tests/character_appearance_oracle.gd")
const Pixels := preload("res://tests/character_real_pixels.gd")

func _initialize() -> void:
	var oracle := Appearance.new()
	var expected := {
		1: [0, 7, 51], 2: [0, 3, 7, 35, 36, 51],
		3: [0, 7, 16, 35, 39, 51], 4: [0, 3, 34, 37, 39, 51],
		6: [0, 7, 35, 37, 39, 51], 10: [0, 35, 51],
	}
	for race in expected:
		assert(oracle.helmet_hidden_groups(178254, race) == expected[race])
	var parts := [0, 2, 401, 501, 701, 702, 1301, 1801, 2001, 2201, 2701, 2702, 3202, 3301, 5101]
	var app := {"race": 1, "sex": 0, "class": 1, "geosets": [[0, 2], [7, 2]], "displays": {"Head": 178254}}
	assert(oracle.visible_parts(app, parts) == [0, 401, 501, 701, 1301, 1801, 2001, 2201, 2702, 3202, 3301, 5101])
	assert(oracle.errors.is_empty())
	var fixture := Pixels.new()
	var loader: Object = ClassDB.instantiate("WowAssetLoader")
	var blp := FileAccess.get_file_as_bytes("res://../data/textures/143838.blp")
	var offset := blp.decode_u32(20)
	var length := blp.decode_u32(84)
	var compressed := Image.create_from_data(blp.decode_u32(12), blp.decode_u32(16), false, Image.FORMAT_DXT1, blp.slice(offset, offset + length))
	var decoded: Image = loader.load_blp("res://../data/textures/143838.blp").image
	assert(fixture.texture_matches_file(compressed, decoded, 143838))
	assert(not fixture.texture_matches_file(compressed, decoded, 137596))
	fixture.free()
	loader = null
	print("PASS six helmet relationship sets, visible hair/ear/head groups, exact DXT binding and wrong-file rejection")
	quit(0)
