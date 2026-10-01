extends RefCounted

# Independent model of a Retail character's appearance, read straight from the DB2 CSV
# exports and texture files, sharing no code with the native loader under test.
# Rules and their references:
# - Body model: ChrRaceXChrModel -> ChrModel.DisplayID -> CreatureDisplayInfo.ModelID ->
#   CreatureModelData.FileDataID; texture layout ChrModel.CharComponentTextureLayoutID.
# - Customization: WMVx CharacterCustomization.cpp ModernCharacterCustomizationProvider::
#   updateContext (an element applies when its related choice is selected; geoset,
#   material and texture layer per ChrModelTextureLayer of the layout) and ::update.
# - Canvases: one per ChrModelMaterial (layout, M2 texture type) at its Width x Height;
#   layers in Layer order (WMVx CharacterTextureBuilder::build), TextureSectionTypeBitMask
#   -1 = whole canvas, else each set bit's CharComponentTextureSections rect.
# - Blend modes: WMVx CharacterTextureBuilder::BlendMode (1 blit, 4 multiply, 6 overlay,
#   7 screen, 9 straight alpha, 15 inferred alpha).
# - Region paste and scaling: solarityclient character_component/composer.rs
#   (authored mip whose width fits, or Wow.exe's exact 2x PasteScale; alpha_blend >> 8).
#   HD sections need 4x for legacy item files: repeated PasteScale (no stock reference).
# - Item textures: solarityclient character_component/atlas.rs ITEM_PRIORITIES and
#   adjusted_item_priority (build 12340 CCharacterComponent), pasted after customization.
# - Item files by race/sex: ComponentTextureFileData (GenderIndex 0 male, 1 female,
#   2/3 either; ClassID 0 any) along the ChrRaces texture fallback chain, then race 0.
# - Geosets: WMVx ModelDefaultsGeosetModifier (0 and every x01), CharacterDefaults (ears
#   702), ModernCharCustomGeosetModifier (customization type/id, face 3202),
#   CharEyeGlowGeosetBasedGeosetModifier (group 17 cleared unless a death knight), then
#   solarityclient geoset.rs apply_equipment_geosets order with wowdev.wiki
#   DB/ItemDisplayInfo modern groups (gloves [1] 23, chest [3] 22 / [4] 28, boots [1]
#   20: 2002 for 0, else 2000 + value).

const DATA := "res://../data/"
const BUILD := "res://../data/db2/12.1.0.69933/"
const DEATH_KNIGHT := 6

# Build 12340 CCharacterComponent paste priority: rows head, shoulder, shirt, chest,
# waist, legs, feet, wrist, hands, tabard; columns ArmUpper, ArmLower, Hand, TorsoUpper,
# TorsoLower, LegUpper, LegLower, Foot.
const ITEM_PRIORITIES := [
	[-1, -1, -1, -1, -1, -1, -1, -1],
	[-1, -1, -1, -1, -1, -1, -1, -1],
	[0, 0, -1, 0, 0, -1, -1, -1],
	[1, 1, -1, 1, 1, 1, 1, -1],
	[-1, -1, -1, -1, 5, 2, -1, -1],
	[-1, -1, -1, -1, -1, 0, 0, -1],
	[-1, -1, -1, -1, -1, -1, 2, 0],
	[-1, 2, -1, -1, -1, -1, -1, -1],
	[-1, 3, 0, -1, -1, -1, -1, -1],
	[-1, -1, -1, 4, 4, -1, -1, -1],
]
const COMPONENT_ROWS := {"Head": 0, "Shoulder": 1, "Shirt": 2, "Chest": 3, "Waist": 4, "Legs": 5, "Feet": 6, "Wrist": 7, "Hands": 8, "Tabard": 9}

var tables := {}
var errors: Array[String] = []

# --- tables ------------------------------------------------------------------------------

# The build's own export (db2/12.1.0.69933) when it has every column, else data/'s.
func table_path(name: String, columns: Array) -> String:
	var build := BUILD + name + ".csv"
	if FileAccess.file_exists(build):
		var header := FileAccess.open(build, FileAccess.READ).get_csv_line()
		if columns.all(func(column): return header.has(column)):
			return build
	return DATA + name + ".csv"

# Rows of `name` as dictionaries of the named columns (ints), optionally indexed by one.
func rows(name: String, columns: Array, key: String = "") -> Variant:
	var cache_key := name + ":" + ",".join(columns) + ":" + key
	if tables.has(cache_key):
		return tables[cache_key]
	var file := FileAccess.open(table_path(name, columns), FileAccess.READ)
	assert(file != null, "missing table " + name)
	var header := file.get_csv_line()
	var indexes := []
	for column in columns:
		var at := header.find(column)
		assert(at >= 0, "%s has no column %s" % [name, column])
		indexes.append(at)
	var result: Variant = {} if key != "" else []
	while not file.eof_reached():
		var line := file.get_csv_line()
		if line.size() < header.size():
			continue
		var row := {}
		for i in columns.size():
			row[columns[i]] = int(line[indexes[i]])
		if key == "":
			result.append(row)
		else:
			if not result.has(row[key]):
				result[row[key]] = []
			result[row[key]].append(row)
	tables[cache_key] = result
	return result

func first(name: String, columns: Array, key: String, value: int) -> Dictionary:
	var found: Array = rows(name, columns, key).get(value, [])
	return found[0] if not found.is_empty() else {}

# --- body ----------------------------------------------------------------------------------

func chr_model(race: int, sex: int) -> Dictionary:
	for row in rows("ChrRaceXChrModel", ["ChrRacesID", "ChrModelID", "Sex"], "ChrRacesID").get(race, []):
		if row.Sex == sex:
			return first("ChrModel", ["ID", "DisplayID", "CharComponentTextureLayoutID"], "ID", row.ChrModelID)
	return {}

func body_model_fdid(model: Dictionary) -> int:
	var display := first("CreatureDisplayInfo", ["ID", "ModelID"], "ID", model.DisplayID)
	return first("CreatureModelData", ["ID", "FileDataID"], "ID", display.ModelID).FileDataID

# --- customization ----------------------------------------------------------------------

func options(model_id: int) -> Array:
	var found: Array = rows("ChrCustomizationOption", ["ID", "ChrModelID", "OrderIndex", "Requirement"], "ChrModelID").get(model_id, []).duplicate()
	found.sort_custom(func(a, b): return a.OrderIndex < b.OrderIndex or (a.OrderIndex == b.OrderIndex and a.ID < b.ID))
	return found

func class_allows(requirement: int, class_id: int) -> bool:
	if requirement == 0:
		return true
	var req := first("ChrCustomizationReq", ["ID", "ClassMask"], "ID", requirement)
	return req.is_empty() or req.ClassMask == -1 or req.ClassMask & (1 << (class_id - 1)) != 0

# The option's choices available to `class_id`, in OrderIndex order.
func choices(option_id: int, class_id: int) -> Array:
	var found := []
	for row in rows("ChrCustomizationChoice", ["ID", "ChrCustomizationOptionID", "ChrCustomizationReqID", "OrderIndex"], "ChrCustomizationOptionID").get(option_id, []):
		if class_allows(row.ChrCustomizationReqID, class_id):
			found.append(row)
	found.sort_custom(func(a, b): return a.OrderIndex < b.OrderIndex or (a.OrderIndex == b.OrderIndex and a.ID < b.ID))
	return found

# A choice for every option of the body that has one: `picks` by option ID, else the
# first choice in OrderIndex order (Retail's character-creation default).
func choice_ids(race: int, sex: int, class_id: int, picks: Dictionary) -> Dictionary:
	var model := chr_model(race, sex)
	var result := {}
	for option in options(model.ID):
		if not class_allows(option.Requirement, class_id):
			continue
		var available := choices(option.ID, class_id)
		if available.is_empty():
			continue
		if picks.has(option.ID):
			result[option.ID] = picks[option.ID]
			continue
		result[option.ID] = available[0].ID
	return result

# The choices that change the render: some element with a material, geoset, or a
# skinned model showing a mesh (GeosetID 0 shows none, e.g. Blindfold "None").
func visual_choices(chosen: Dictionary) -> Dictionary:
	var result := {}
	for option in chosen:
		for element in elements(chosen[option]):
			var model := first("ChrCustomizationSkinnedModel", ["ID", "GeosetID"], "ID", element.ChrCustomizationSkinnedModelID)
			if element.ChrCustomizationMaterialID != 0 or element.ChrCustomizationGeosetID != 0 or (not model.is_empty() and model.GeosetID != 0):
				result[option] = chosen[option]
	return result

func elements(choice_id: int) -> Array:
	return rows("ChrCustomizationElement", ["ChrCustomizationChoiceID", "RelatedChrCustomizationChoiceID", "ChrCustomizationGeosetID", "ChrCustomizationSkinnedModelID", "ChrCustomizationMaterialID"], "ChrCustomizationChoiceID").get(choice_id, [])

func texture_fdids(material_resources: int) -> Array:
	var found := []
	for row in rows("TextureFileData", ["FileDataID", "MaterialResourcesID"], "MaterialResourcesID").get(material_resources, []):
		found.append(row.FileDataID)
	return found

# Selected materials [[target, fdid]], geosets [[type, id]] and skinned model IDs.
func customization(chosen: Dictionary, race: int, sex: int, class_id: int) -> Dictionary:
	var selected: Array = chosen.values()
	var result := {"materials": [], "geosets": [], "skinned": []}
	for choice_id in selected:
		for element in elements(choice_id):
			if element.RelatedChrCustomizationChoiceID != 0 and not selected.has(element.RelatedChrCustomizationChoiceID):
				continue
			if element.ChrCustomizationGeosetID != 0:
				var geoset := first("ChrCustomizationGeoset", ["ID", "GeosetType", "GeosetID"], "ID", element.ChrCustomizationGeosetID)
				result.geosets.append([geoset.GeosetType, geoset.GeosetID])
			if element.ChrCustomizationSkinnedModelID != 0:
				result.skinned.append(element.ChrCustomizationSkinnedModelID)
			if element.ChrCustomizationMaterialID != 0:
				var material := first("ChrCustomizationMaterial", ["ID", "ChrModelTextureTargetID", "MaterialResourcesID"], "ID", element.ChrCustomizationMaterialID)
				var fdid := item_texture_fdid(material.MaterialResourcesID, race, sex, class_id)
				if fdid == 0:
					errors.append("material %d resources %d has no file for this body" % [material.ID, material.MaterialResourcesID])
				else:
					result.materials.append([material.ChrModelTextureTargetID, fdid])
	return result

# --- items -------------------------------------------------------------------------------

func item_display(item_id: int) -> int:
	var best := {}
	for row in rows("ItemModifiedAppearance", ["ItemID", "ItemAppearanceModifierID", "ItemAppearanceID", "OrderIndex"], "ItemID").get(item_id, []):
		if row.ItemAppearanceModifierID == 0 and (best.is_empty() or row.OrderIndex < best.OrderIndex):
			best = row
	if best.is_empty():
		return 0
	return first("ItemAppearance", ["ID", "ItemDisplayInfoID"], "ID", best.ItemAppearanceID).ItemDisplayInfoID

func display_info(display_id: int) -> Dictionary:
	return first("ItemDisplayInfo", ["ID", "GeosetGroup_0", "GeosetGroup_1", "GeosetGroup_2", "GeosetGroup_3", "GeosetGroup_4", "GeosetGroup_5", "ModelMaterialResourcesID_0"], "ID", display_id)

func geoset_group(display: Dictionary, index: int) -> int:
	return 0 if display.is_empty() else display["GeosetGroup_%d" % index]

# (race, sex) then each ChrRaces texture fallback, without repeats.
func texture_race_chain(race: int, sex: int) -> Array:
	var chain := [[race, sex]]
	while true:
		var current: Array = chain[chain.size() - 1]
		var row := first("ChrRaces", ["ID", "MaleTextureFallbackRaceID", "MaleTextureFallbackSex", "FemaleTextureFallbackRaceID", "FemaleTextureFallbackSex"], "ID", current[0])
		if row.is_empty():
			break
		var next := [row.MaleTextureFallbackRaceID, row.MaleTextureFallbackSex] if current[1] == 0 else [row.FemaleTextureFallbackRaceID, row.FemaleTextureFallbackSex]
		if next[0] == 0 or chain.has(next):
			break
		chain.append(next)
	return chain

func gender_matches(index: int, sex: int) -> bool:
	return index == sex or index == 2 or index == 3

# The file of `material_resources` a `race`/`sex`/`class_id` character wears.
func item_texture_fdid(material_resources: int, race: int, sex: int, class_id: int) -> int:
	var files := texture_fdids(material_resources)
	var owned := []
	var unowned := []
	for fdid in files:
		var info := first("ComponentTextureFileData", ["ID", "GenderIndex", "ClassID", "RaceID"], "ID", fdid)
		if info.is_empty():
			unowned.append(fdid)
		elif info.ClassID == 0 or info.ClassID == class_id:
			owned.append(info)
	for wanted in texture_race_chain(race, sex) + [[0, sex]]:
		for info in owned:
			if info.RaceID == wanted[0] and gender_matches(info.GenderIndex, wanted[1]):
				return info.ID
	return unowned[0] if not unowned.is_empty() else 0

# Item textures [[section, fdid, priority]] by slot name, and each slot's display.
func equipment(items: Array, race: int, sex: int, class_id: int) -> Dictionary:
	var displays := {}
	for item in items:
		displays[item.slot] = item_display(item.item_id)
	var layers := {}
	for slot in displays:
		if not COMPONENT_ROWS.has(slot):
			continue
		var row: int = COMPONENT_ROWS[slot]
		var display := display_info(displays[slot])
		for material in rows("ItemDisplayInfoMaterialRes", ["ComponentSection", "MaterialResourcesID", "ItemDisplayInfoID"], "ItemDisplayInfoID").get(displays[slot], []):
			var section: int = material.ComponentSection
			if section > 7 or ITEM_PRIORITIES[row][section] < 0:
				continue
			var priority := adjusted_priority(row, section, ITEM_PRIORITIES[row][section], display)
			var fdid := item_texture_fdid(material.MaterialResourcesID, race, sex, class_id)
			if fdid == 0:
				continue
			# One texture per section and priority: a later item replaces the cell.
			layers["%d:%d" % [section, priority]] = [section, fdid, priority]
	var textures := layers.values()
	textures.sort_custom(func(a, b): return a[0] < b[0] or (a[0] == b[0] and a[2] < b[2]))
	var cape := 0
	if displays.has("Back"):
		var back := display_info(displays.Back)
		if not back.is_empty() and back.ModelMaterialResourcesID_0 != 0:
			cape = item_texture_fdid(back.ModelMaterialResourcesID_0, race, sex, class_id)
	return {"displays": displays, "textures": textures, "cape": cape}

func adjusted_priority(row: int, section: int, base: int, display: Dictionary) -> int:
	if row == 3 and section == 1 and geoset_group(display, 0) != 0:
		return 5
	if row == 8 and section == 1 and geoset_group(display, 0) != 0:
		return 6
	if row == 3 and section == 6 and geoset_group(display, 2) != 0:
		return 4
	if row == 6 and section == 6 and geoset_group(display, 0) != 0:
		return 3
	return base

# --- the whole appearance --------------------------------------------------------------

func appearance(race: int, sex: int, class_id: int, picks: Dictionary, items: Array) -> Dictionary:
	var model := chr_model(race, sex)
	var chosen := choice_ids(race, sex, class_id, picks)
	var custom := customization(chosen, race, sex, class_id)
	var gear := equipment(items, race, sex, class_id)
	return {
		"race": race, "sex": sex, "class": class_id, "chr_model": model.ID,
		"layout": model.CharComponentTextureLayoutID, "model_fdid": body_model_fdid(model),
		"choices": chosen, "materials": custom.materials, "geosets": custom.geosets,
		"skinned": custom.skinned, "displays": gear.displays, "item_textures": gear.textures,
		"cape": gear.cape,
	}

# --- geosets -------------------------------------------------------------------------------

# Whether `part` is drawn, over the model's `parts`.
func visible_parts(app: Dictionary, parts: Array) -> Array:
	var visible := {}
	for part in parts:
		if part == 0 or (part > 100 and part % 100 == 1):
			visible[part] = true
	set_group(visible, parts, 7, 702)
	for geoset in app.geosets:
		set_group(visible, parts, geoset[0], geoset[0] * 100 + geoset[1])
	# WMVx forces the face (CG_FACE 1, relative: 3202) after the customization geosets; a
	# face shape choice of the body already selects one.
	if not app.geosets.any(func(geoset): return geoset[0] == 32):
		set_group(visible, parts, 32, 3202)
	if app["class"] != DEATH_KNIGHT:
		hide_range(visible, 1700, 1799)
	apply_equipment(visible, parts, app.displays)
	var result := []
	for part in parts:
		if visible.has(part) and not result.has(part):
			result.append(part)
	result.sort()
	return result

# WMVx GeosetState::setVisibility: within the group (ids group*100+1..+99, ids 1..99
# for group 0) exactly `id` is shown.
func set_group(visible: Dictionary, parts: Array, group: int, id: int) -> void:
	for part in parts:
		if part > group * 100 and part < group * 100 + 100:
			if part == id:
				visible[part] = true
			else:
				visible.erase(part)

func hide_range(visible: Dictionary, low: int, high: int) -> void:
	for part in visible.keys():
		if part >= low and part <= high:
			visible.erase(part)

func show(visible: Dictionary, parts: Array, part: int) -> void:
	if parts.has(part):
		visible[part] = true

func apply_equipment(visible: Dictionary, parts: Array, displays: Dictionary) -> void:
	var shirt := display_info(displays.get("Shirt", 0))
	var chest := display_info(displays.get("Chest", 0))
	var waist := display_info(displays.get("Waist", 0))
	var legs := display_info(displays.get("Legs", 0))
	var feet := display_info(displays.get("Feet", 0))
	var hands := display_info(displays.get("Hands", 0))
	var tabard := display_info(displays.get("Tabard", 0))
	var cape := display_info(displays.get("Back", 0))
	var wrist := display_info(displays.get("Wrist", 0))
	if geoset_group(hands, 0) != 0:
		hide_range(visible, 401, 499)
		show(visible, parts, 401 + geoset_group(hands, 0))
	elif geoset_group(chest, 0) != 0:
		show(visible, parts, 801 + geoset_group(chest, 0))
	if geoset_group(hands, 1) != 0:
		set_group(visible, parts, 23, 2301 + geoset_group(hands, 1))
	var outer_arm := false
	for display_id in [displays.get("Chest", 0), displays.get("Wrist", 0), displays.get("Hands", 0)]:
		for material in rows("ItemDisplayInfoMaterialRes", ["ComponentSection", "MaterialResourcesID", "ItemDisplayInfoID"], "ItemDisplayInfoID").get(display_id, []):
			if material.ComponentSection == 1:
				outer_arm = true
	if not outer_arm and geoset_group(shirt, 0) != 0:
		show(visible, parts, 801 + geoset_group(shirt, 0))
	var chest_robe := geoset_group(chest, 2)
	var legs_robe := geoset_group(legs, 2) if chest_robe == 0 else 0
	var robe := chest_robe if chest_robe != 0 else legs_robe
	if robe != 0:
		hide_range(visible, 501, 599)
		hide_range(visible, 902, 999)
		hide_range(visible, 1100, 1199)
		hide_range(visible, 1300, 1399)
		show(visible, parts, 1301 + robe)
	else:
		if geoset_group(feet, 0) != 0:
			hide_range(visible, 501, 599)
			show(visible, parts, 901)
			show(visible, parts, 501 + geoset_group(feet, 0))
		show(visible, parts, 901 + geoset_group(legs, 1))
	# wowdev.wiki DB/ItemDisplayInfo: worn boots give 2002 for [1] = 0, else 2000 + [1].
	if not feet.is_empty():
		set_group(visible, parts, 20, 2002 if geoset_group(feet, 1) == 0 else 2000 + geoset_group(feet, 1))
	var item_tabard := false
	if robe == 0 and geoset_group(tabard, 0) != 0:
		show(visible, parts, 1201 + geoset_group(tabard, 0))
		item_tabard = true
	if chest_robe == 0:
		if not item_tabard and geoset_group(shirt, 1) != 0:
			show(visible, parts, 1001 + geoset_group(shirt, 1))
		var trousers := geoset_group(legs, 0)
		if trousers != 0 and (trousers >= 3 or not item_tabard):
			if trousers >= 3:
				hide_range(visible, 1300, 1399)
			show(visible, parts, 1101 + trousers)
	if geoset_group(chest, 1) != 0:
		set_group(visible, parts, 10, 1001 + geoset_group(chest, 1))
	if geoset_group(chest, 3) != 0:
		set_group(visible, parts, 22, 2201 + geoset_group(chest, 3))
	if geoset_group(chest, 4) != 0:
		set_group(visible, parts, 28, 2801 + geoset_group(chest, 4))
	if geoset_group(cape, 0) != 0:
		hide_range(visible, 1500, 1599)
		show(visible, parts, 1501 + geoset_group(cape, 0))
	if geoset_group(waist, 0) != 0:
		hide_range(visible, 1800, 1899)
		show(visible, parts, 1801 + geoset_group(waist, 0))

# --- texture canvases ------------------------------------------------------------------

func section_rect(layout: int, section: int) -> Rect2i:
	for row in rows("CharComponentTextureSections", ["CharComponentTextureLayoutID", "SectionType", "X", "Y", "Width", "Height"], "CharComponentTextureLayoutID").get(layout, []):
		if row.SectionType == section:
			return Rect2i(row.X, row.Y, row.Width, row.Height)
	return Rect2i()

func canvas_size(layout: int, texture_type: int) -> Vector2i:
	for row in rows("ChrModelMaterial", ["CharComponentTextureLayoutsID", "TextureType", "Width", "Height"], "CharComponentTextureLayoutsID").get(layout, []):
		if row.TextureType == texture_type:
			return Vector2i(row.Width, row.Height)
	return Vector2i.ZERO

func layers(layout: int, texture_type: int) -> Array:
	var found := []
	for row in rows("ChrModelTextureLayer", ["TextureType", "Layer", "BlendMode", "TextureSectionTypeBitMask", "ChrModelTextureTargetID_0", "CharComponentTextureLayoutsID"], "CharComponentTextureLayoutsID").get(layout, []):
		if row.TextureType == texture_type:
			found.append(row)
	found.sort_custom(func(a, b): return a.Layer < b.Layer)
	return found

# The composed canvas of M2 texture `texture_type`, or null when no selected material
# reaches one of its layers. `load_image` maps an FDID to an RGBA8 Image (or null).
func canvas(app: Dictionary, texture_type: int, load_image: Callable) -> Variant:
	var size := canvas_size(app.layout, texture_type)
	if size == Vector2i.ZERO:
		return null
	var image := Image.create_empty(size.x, size.y, false, Image.FORMAT_RGBA8)
	var used := false
	for layer in layers(app.layout, texture_type):
		for material in app.materials:
			if material[0] != layer.ChrModelTextureTargetID_0:
				continue
			var source: Variant = load_image.call(material[1])
			if source == null:
				errors.append("texture %d of target %d unavailable" % [material[1], material[0]])
				continue
			used = true
			for rect in layer_rects(app.layout, layer.TextureSectionTypeBitMask, size):
				paste(image, source, rect, layer.BlendMode)
	if texture_type == 1:
		for texture in app.item_textures:
			var source: Variant = load_image.call(texture[1])
			if source == null:
				errors.append("item texture %d unavailable" % texture[1])
				continue
			used = true
			paste(image, source, section_rect(app.layout, texture[0]), 15)
	return image if used else null

func layer_rects(layout: int, mask: int, size: Vector2i) -> Array:
	if mask == -1:
		return [Rect2i(Vector2i.ZERO, size)]
	var rects := []
	for bit in 32:
		if mask & (1 << bit) != 0:
			var rect := section_rect(layout, bit)
			if rect.size != Vector2i.ZERO:
				rects.append(rect)
	return rects

# The source as solarity's region paste reads it at the rect's size: the authored mip
# whose width fits (2x2 box mips), or the exact 2x PasteScale expansion.
func fitted(source: Image, size: Vector2i) -> Image:
	var level := source.duplicate() as Image
	level.convert(Image.FORMAT_RGBA8)
	# Wow.exe expands one level per PasteScale; the HD atlas needs 4x for legacy item
	# files, taken here as repeated 2x expansion (assumption, no stock reference).
	while level.get_width() < size.x and level.get_height() < size.y:
		if level.get_width() * 2 > size.x or level.get_height() * 2 > size.y:
			errors.append("source %s cannot scale to %s" % [level.get_size(), size])
			return null
		level = paste_scale(level)
	while level.get_width() > size.x:
		level.resize(maxi(level.get_width() / 2, 1), maxi(level.get_height() / 2, 1), Image.INTERPOLATE_BILINEAR)
	if level.get_width() != size.x or level.get_height() < size.y:
		errors.append("source mip %s does not cover %s" % [level.get_size(), size])
		return null
	return level

# Wow.exe 0x004EF9D0 PasteScale (composer.rs blend_scaled_rect): even texels copy,
# odd ones average their neighbours, truncating.
static func paste_scale(source: Image) -> Image:
	var w := source.get_width()
	var h := source.get_height()
	var src := source.get_data()
	var out := PackedByteArray()
	out.resize(w * 2 * h * 2 * 4)
	for y in h * 2:
		var sy := y >> 1
		var ny := mini(sy + 1, h - 1)
		for x in w * 2:
			var sx := x >> 1
			var nx := mini(sx + 1, w - 1)
			var o := (y * w * 2 + x) * 4
			var a := (sy * w + sx) * 4
			var b := (sy * w + nx) * 4
			var c := (ny * w + sx) * 4
			var d := (ny * w + nx) * 4
			var odd_x := x & 1 == 1
			var odd_y := y & 1 == 1
			for ch in 4:
				if odd_x and odd_y:
					out[o + ch] = (src[a + ch] + src[b + ch] + src[c + ch] + src[d + ch]) >> 2
				elif odd_x:
					out[o + ch] = (src[a + ch] + src[b + ch]) >> 1
				elif odd_y:
					out[o + ch] = (src[a + ch] + src[c + ch]) >> 1
				else:
					out[o + ch] = src[a + ch]
	return Image.create_from_data(w * 2, h * 2, false, Image.FORMAT_RGBA8, out)

func paste(image: Image, source: Image, rect: Rect2i, blend_mode: int) -> void:
	var fit := fitted(source, rect.size)
	if fit == null:
		return
	var src := fit.get_data()
	var dst := image.get_data()
	var src_width := fit.get_width()
	var width := image.get_width()
	for y in rect.size.y:
		var s := y * src_width * 4
		var d := ((rect.position.y + y) * width + rect.position.x) * 4
		for x in rect.size.x:
			blend_texel(dst, d + x * 4, src, s + x * 4, blend_mode)
	image.set_data(image.get_width(), image.get_height(), false, Image.FORMAT_RGBA8, dst)

static func tint(mode: int, s: int, d: int) -> int:
	match mode:
		4: return s * d / 255
		6: return 2 * s * d / 255 if d < 128 else 255 - 2 * (255 - s) * (255 - d) / 255
		7: return 255 - (255 - s) * (255 - d) / 255
	return s

static func blend_texel(dst: PackedByteArray, d: int, src: PackedByteArray, s: int, mode: int) -> void:
	var alpha := src[s + 3]
	if mode == 0 or mode == 1:
		for ch in 4:
			dst[d + ch] = src[s + ch]
		return
	if dst[d + 3] < 255 and (mode == 9 or mode == 15):
		# QPainter SourceOver onto a translucent canvas (WMVx mergeLayer): the
		# destination weighs by its own alpha.
		var below := dst[d + 3] * (255 - alpha)
		var total := alpha * 255 + below
		if total == 0:
			return
		for ch in 3:
			dst[d + ch] = (src[s + ch] * alpha * 255 + dst[d + ch] * below) / total
		dst[d + 3] = total / 255
		return
	for ch in 3:
		var value := tint(mode, src[s + ch], dst[d + ch])
		dst[d + ch] = (value * alpha + dst[d + ch] * (255 - alpha)) >> 8
	dst[d + 3] = maxi(dst[d + 3], alpha)
