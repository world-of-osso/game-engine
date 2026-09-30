#!/usr/bin/env python3
"""Export the columns the client reads from local-CASC WDC5 tables to data/db2 CSVs.

Only the layouts listed in TABLES (build 12.1.0.69933) are accepted. Sections whose
TACT key is unknown arrive zero-filled from casc-local; their records are dropped and
counted on stderr.

Usage: export_db2_csv.py <table> <file.db2> <out.csv>
  ChrCustomizationReq          FDID 3450453
  ChrCustomizationReqChoice    FDID 3580359
  Emotes                       FDID 1343602
  JournalInstance              FDID 1237438 (localized: pass its enUS copy)
  JournalInstanceEntrance      FDID 5228481
  NPCModelItemSlotDisplayInfo  FDID 1340661
  UiTextureKit                 FDID 939159
  ZoneLight                    FDID 1310253
  ZoneLightPoint               FDID 1310256
  AnimationData                FDID 1375431
  AnimKitBoneSet               FDID 1375433
  AnimKitConfig                FDID 1300872
  AnimKitConfigBoneSet         FDID 1300873
  AnimKitPriority              FDID 1266540
  AnimKitSegment               FDID 1304324
  SpellVisual                  FDID 897952
  SpellVisualAnim              FDID 1140479
  SpellVisualEffectName        FDID 897948
  SpellVisualEvent             FDID 1685317
  SpellVisualKit               FDID 897949
  SpellVisualKitEffect         FDID 1140480
  SpellVisualKitModelAttach    FDID 897953
  SpellVisualMissile           FDID 897954
  SpellXSpellVisual            FDID 1101657
  SoundKit                     FDID 1237434
  SoundKitEntry                FDID 1237435
  CreatureSoundData            FDID 1344466
  WeaponSwingSounds2           FDID 1267068
  WeaponImpactSounds           FDID 1267648
  ChrModel                     FDID 3384313
  ChrRaceXChrModel             FDID 3490304
  ChrCustomizationElement      FDID 3512765
  ChrCustomizationMaterial     FDID 3459652
  ChrCustomizationSkinnedModel FDID 3460183
  ChrModelTextureLayer         FDID 3548976
  LightParams                  FDID 1334669
  LiquidType                   FDID 1371380
  LiquidMaterial               FDID 1132538
  LiquidObject                 FDID 1308058
  LiquidTypeXTexture           FDID 2261065
"""

import csv
import struct
import sys

# (layout hash, [(CSV column, source)]); source is "id", "parent", a field index,
# ("string", field index) for an inline string field of a single-section table,
# ("float", field index, element) for 32-bit element `element` of a float field, or
# ("int", field index, element) for signed 32-bit element `element` of an integer field.
TABLES = {
    # WoWDBDefs layout CA154412: ReqSource_lang (field 0) is not exported.
    "ChrCustomizationReq": (
        0xCA154412,
        [
            ("ID", "id"),
            ("ReqType", ("int", 1, 0)),
            ("ClassMask", ("int", 2, 0)),
            ("RegionGroupMask", ("int", 3, 0)),
            ("ReqAchievementID", ("int", 4, 0)),
            ("ReqQuestID", ("int", 5, 0)),
            ("OverrideArchive", ("int", 6, 0)),
            ("ReqItemModifiedAppearanceID", ("int", 7, 0)),
            ("RaceMasks_0", ("int", 8, 0)),
            ("RaceMasks_1", ("int", 8, 1)),
        ],
    ),
    # WoWDBDefs layout F925BC6F: the requirement is the relation (parent) column.
    "ChrCustomizationReqChoice": (
        0xF925BC6F,
        [("ID", "id"), ("ChrCustomizationChoiceID", 0), ("ChrCustomizationReqID", "parent")],
    ),
    "Emotes": (0x0A598B68, [("ID", "id"), ("AnimID", 1)]),
    "JournalInstance": (
        0x6C5ED7F2,
        [("ID", "id"), ("Name_lang", ("string", 0)), ("MapID", 2), ("Flags", 7), ("AreaID", 8)],
    ),
    "JournalInstanceEntrance": (
        0x874E7CC2,
        [
            ("ID", "id"),
            ("Pos_0", ("float", 0, 0)),
            ("Pos_1", ("float", 0, 1)),
            ("Pos_2", ("float", 0, 2)),
            ("MapID", 1),
            ("AreaTableID", 2),
            ("Faction", 3),
            ("JournalInstanceID", "parent"),
        ],
    ),
    "NPCModelItemSlotDisplayInfo": (
        0xC2057F5B,
        [("ID", "id"), ("NpcModelID", "parent"), ("ItemDisplayInfoID", 0), ("ItemSlot", 1)],
    ),
    "UiTextureKit": (0x4740638A, [("ID", "id"), ("KitPrefix", ("string", 0))]),
    "ZoneLight": (
        0x94CE95E0,
        [
            ("ID", "id"),
            ("Name", ("string", 0)),
            ("MapID", 1),
            ("LightID", 2),
            ("Flags", 3),
            ("Zmin", ("float", 4, 0)),
            ("Zmax", ("float", 5, 0)),
            ("TransitionType", 6),
            ("PlayerConditionID", 7),
        ],
    ),
    "ZoneLightPoint": (
        0xDE2377FB,
        [
            ("ID", "id"),
            ("Pos_0", ("float", 0, 0)),
            ("Pos_1", ("float", 0, 1)),
            ("PointOrder", 1),
            ("ZoneLightID", "parent"),
        ],
    ),
    # Spell visuals (WoWDBDefs layouts of build 12.1.0.69933). Floats are exported as
    # ("float", field, element); `<8>` fields are signed, so AttachmentID -1 stays -1.
    "AnimationData": (0xBBF66A3C, [("ID", "id"), ("Fallback", ("u16", 0)), ("BehaviorTier", ("i8", 1)), ("BehaviorID", ("i16", 2)), ("Flags_0", ("int", 3, 0))]),
    "SpellVisual": (
        0x4B85C90F,
        [
            ("ID", "id"),
            ("MissileCastOffset_0", ("float", 0, 0)),
            ("MissileCastOffset_1", ("float", 0, 1)),
            ("MissileCastOffset_2", ("float", 0, 2)),
            ("MissileImpactOffset_0", ("float", 1, 0)),
            ("MissileImpactOffset_1", ("float", 1, 1)),
            ("MissileImpactOffset_2", ("float", 1, 2)),
            ("Flags", ("int", 4, 0)),
            ("MissileAttachment", ("i8", 5)),
            ("MissileDestinationAttachment", ("i8", 6)),
            ("SpellVisualMissileSetID", ("u16", 12)),
        ],
    ),
    "SpellVisualAnim": (0xF233613A, [("ID", "id"), ("InitialAnimID", ("i16", 0)), ("LoopAnimID", ("i16", 1)), ("AnimKitID", ("u16", 2))]),
    "SpellVisualEffectName": (
        0x2245CEE6,
        [
            ("ID", "id"),
            ("ModelFileDataID", ("int", 0, 0)),
            ("BaseMissileSpeed", ("float", 1, 0)),
            ("Scale", ("float", 2, 0)),
            ("MinAllowedScale", ("float", 3, 0)),
            ("MaxAllowedScale", ("float", 4, 0)),
            ("Alpha", ("float", 5, 0)),
            ("Flags", ("int", 6, 0)),
            ("TextureFileDataID", ("int", 7, 0)),
            ("Type", ("int", 9, 0)),
            ("GenericID", ("int", 10, 0)),
        ],
    ),
    "SpellVisualEvent": (
        0x865F512E,
        [
            ("ID", "id"),
            ("StartEvent", ("int", 0, 0)),
            ("EndEvent", ("int", 1, 0)),
            ("StartMinOffsetMs", ("int", 2, 0)),
            ("StartMaxOffsetMs", ("int", 3, 0)),
            ("EndMinOffsetMs", ("int", 4, 0)),
            ("EndMaxOffsetMs", ("int", 5, 0)),
            ("TargetType", ("int", 6, 0)),
            ("SpellVisualKitID", ("int", 7, 0)),
            ("SpellVisualID", "parent"),
        ],
    ),
    "SpellVisualKit": (
        0xC069D9C4,
        [("ID", "id"), ("FallbackSpellVisualKitID", ("int", 1, 0)), ("DelayMin", ("u16", 2)), ("DelayMax", ("u16", 3)), ("Flags_0", ("int", 8, 0))],
    ),
    "SpellVisualKitEffect": (
        0xE3206CA2,
        [("ID", "id"), ("EffectType", ("int", 0, 0)), ("Effect", ("int", 1, 0)), ("ParentSpellVisualKitID", "parent")],
    ),
    "SpellVisualKitModelAttach": (
        0x02CF8554,
        [
            ("ID", "id"),
            ("Offset_0", ("float", 0, 0)),
            ("Offset_1", ("float", 0, 1)),
            ("Offset_2", ("float", 0, 2)),
            ("SpellVisualEffectNameID", ("int", 2, 0)),
            ("AttachmentID", ("i8", 3)),
            ("PositionerID", ("int", 4, 0)),
            ("Yaw", ("float", 5, 0)),
            ("Pitch", ("float", 6, 0)),
            ("Roll", ("float", 7, 0)),
            ("Scale", ("float", 11, 0)),
            ("StartAnimID", ("i16", 13)),
            ("AnimID", ("i16", 14)),
            ("EndAnimID", ("i16", 15)),
            ("AnimKitID", ("int", 16, 0)),
            ("Flags", ("int", 17, 0)),
            ("StartDelay", ("float", 19, 0)),
            ("ParentSpellVisualKitID", "parent"),
        ],
    ),
    "SpellVisualMissile": (
        0xAE389078,
        [
            ("ID", "id"),
            ("CastOffset_0", ("float", 0, 0)),
            ("CastOffset_1", ("float", 0, 1)),
            ("CastOffset_2", ("float", 0, 2)),
            ("ImpactOffset_0", ("float", 1, 0)),
            ("ImpactOffset_1", ("float", 1, 1)),
            ("ImpactOffset_2", ("float", 1, 2)),
            ("SpellVisualEffectNameID", ("u16", 3)),
            ("SoundEntriesID", 4),
            ("Attachment", ("i8", 5)),
            ("DestinationAttachment", ("i8", 6)),
            ("Flags", ("int", 12, 0)),
            ("SpellMissileMotionID", ("u16", 13)),
            ("DecayTimeAfterImpact", ("int", 16, 0)),
            ("SpellVisualMissileSetID", "parent"),
        ],
    ),
    "SpellXSpellVisual": (
        0x7994A890,
        [
            ("ID", "id"),
            ("DifficultyID", ("i16", 1)),
            ("SpellVisualID", 2),
            ("Probability", ("float", 3, 0)),
            ("Priority", ("int", 5, 0)),
            ("ViewerUnitConditionID", ("u16", 8)),
            ("ViewerPlayerConditionID", 9),
            ("CasterUnitConditionID", ("u16", 10)),
            ("CasterPlayerConditionID", 11),
            ("SpellID", "parent"),
        ],
    ),
    "AnimKitSegment": (
        0xA6C970CA,
        [
            ("ID", "id"),
            ("ParentAnimKitID", ("u16", 0)),
            ("OrderIndex", ("u8", 1)),
            ("AnimID", ("i16", 2)),
            ("AnimStartTime", 3),
            ("AnimKitConfigID", ("u16", 4)),
            ("StartCondition", ("u8", 5)),
            ("StartConditionParam", ("u8", 6)),
            ("StartConditionDelay", 7),
            ("EndCondition", ("u8", 8)),
            ("EndConditionParam", 9),
            ("EndConditionDelay", 10),
            ("Speed", ("float", 11, 0)),
            ("SegmentFlags", ("int", 12, 0)),
            ("ForcedVariation", ("u8", 13)),
            ("OverrideConfigFlags", ("int", 14, 0)),
            ("LoopToSegmentIndex", ("i8", 15)),
            ("BlendInTimeMs", ("u16", 16)),
            ("BlendOutTimeMs", ("u16", 17)),
        ],
    ),
    "AnimKitConfig": (0x140718EF, [("ID", "id"), ("ConfigFlags", ("int", 0, 0))]),
    "AnimKitConfigBoneSet": (
        0x482E3ED3,
        [("ID", "id"), ("AnimKitBoneSetID", ("u8", 0)), ("AnimKitPriorityID", ("u16", 1)), ("ParentAnimKitConfigID", "parent")],
    ),
    "AnimKitBoneSet": (
        0x43E7736F,
        [("ID", "id"), ("BoneDataID", ("int", 1, 0)), ("ParentAnimKitBoneSetID", ("i8", 2)), ("AltAnimKitBoneSetID", ("i8", 3)), ("AltBoneDataID", ("int", 4, 0))],
    ),
    "AnimKitPriority": (0xCCF889D8, [("ID", "id"), ("Priority", ("u8", 0))]),
    "SoundKit": (
        0xA7FB0451,
        [
            ("ID", "id"),
            ("SoundType", ("int", 0, 0)),
            ("VolumeFloat", ("float", 1, 0)),
            ("Flags", ("int", 2, 0)),
            ("MinDistance", ("float", 3, 0)),
            ("DistanceCutoff", ("float", 4, 0)),
        ],
    ),
    "CreatureSoundData": (
        0xE5EE765B,
        [
            ("ID", "id"),
            ("SoundExertionID", 1),
            ("SoundExertionCriticalID", 2),
            ("SoundInjuryID", 3),
            ("SoundInjuryCriticalID", 4),
            ("SoundDeathID", 6),
            ("SpellCastDirectedSoundID", 21),
            ("WindupSoundID", 24),
            ("WindupCriticalSoundID", 25),
            ("ChargeSoundID", 26),
            ("ChargeCriticalSoundID", 27),
            ("BattleShoutSoundID", 28),
            ("BattleShoutCriticalSoundID", 29),
            ("TauntSoundID", 30),
            ("CreatureImpactType", ("i8", 34)),
        ],
    ),
    # WoWDBDefs layout 8CC18B68 (non-inline ID).
    "WeaponSwingSounds2": (
        0x8CC18B68,
        [("ID", "id"), ("SwingType", ("u8", 0)), ("Crit", ("u8", 1)), ("SoundID", 2)],
    ),
    # WoWDBDefs layout A77CBD9D (non-inline ID); the four sound arrays have 11 elements.
    "WeaponImpactSounds": (
        0xA77CBD9D,
        [("ID", "id"), ("WeaponSubClassID", ("u8", 0)), ("ParrySoundType", ("u8", 1)), ("ImpactSource", ("u8", 2))]
        + [
            (f"{name}_{element}", ("int", field, element))
            for field, name in (
                (3, "ImpactSoundID"),
                (4, "CritImpactSoundID"),
                (5, "PierceImpactSoundID"),
                (6, "PierceCritImpactSoundID"),
            )
            for element in range(11)
        ],
    ),
    # WoWDBDefs layout 03FAB755: the inline ID is field 2.
    "ChrModel": (0x03FAB755, [("ID", "id"), ("Sex", ("u8", 3)), ("DisplayID", 4)]),
    "ChrRaceXChrModel": (
        0xA203BC29,
        [("ID", "id"), ("ChrRacesID", ("u8", 0)), ("ChrModelID", 1), ("Sex", ("u8", 2))],
    ),
    "ChrCustomizationElement": (
        0x6483C37E,
        [("ID", "id")]
        + [
            (name, ("int", index, 0))
            for index, name in enumerate(
                [
                    "ChrCustomizationChoiceID",
                    "RelatedChrCustomizationChoiceID",
                    "ChrCustomizationGeosetID",
                    "ChrCustomizationSkinnedModelID",
                    "ChrCustomizationMaterialID",
                    "ChrCustomizationBoneSetID",
                    "ChrCustomizationCondModelID",
                    "ChrCustomizationDisplayInfoID",
                    "ChrCustItemGeoModifyID",
                    "ChrCustomizationVoiceID",
                    "AnimKitID",
                    "ParticleColorID",
                    "ChrCustGeoComponentLinkID",
                ]
            )
        ],
    ),
    "ChrCustomizationMaterial": (
        0xBE9767E9,
        [("ID", "id"), ("ChrModelTextureTargetID", ("int", 0, 0)), ("MaterialResourcesID", ("int", 1, 0))],
    ),
    "ChrCustomizationSkinnedModel": (
        0x4C32AA8A,
        [
            ("ID", "id"),
            ("CollectionsFileDataID", ("int", 0, 0)),
            ("GeosetType", ("u8", 1)),
            ("GeosetID", ("int", 2, 0)),
            ("Modifier", ("int", 3, 0)),
            ("Flags", ("int", 4, 0)),
        ],
    ),
    # WoWDBDefs layout 22469480: inline ID, then the layout relation as field 1.
    "ChrModelMaterial": (
        0x22469480,
        [
            ("ID", "id"),
            ("CharComponentTextureLayoutsID", ("int", 1, 0)),
            ("TextureType", ("int", 2, 0)),
            ("Width", ("int", 3, 0)),
            ("Height", ("int", 4, 0)),
            ("Flags", ("int", 5, 0)),
        ],
    ),
    "ChrModelTextureLayer": (
        0xD0583FB4,
        [
            ("ID", "id"),
            ("TextureType", ("int", 0, 0)),
            ("Layer", ("int", 1, 0)),
            ("Flags", ("int", 2, 0)),
            ("BlendMode", ("int", 3, 0)),
            ("TextureSectionTypeBitMask", ("int", 4, 0)),
            ("TextureSectionTypeBitMask2", ("int", 5, 0)),
            ("Field_9_0_1_34365_006_0", ("int", 6, 0)),
            ("Field_9_0_1_34365_006_1", ("int", 6, 1)),
            ("Field_9_0_1_34365_006_2", ("int", 6, 2)),
            ("ChrModelTextureTargetID_0", ("int", 7, 0)),
            ("ChrModelTextureTargetID_1", ("int", 7, 1)),
            ("CharComponentTextureLayoutsID", "parent"),
        ],
    ),
    "SoundKitEntry": (
        0x8F82FF7D,
        [
            ("ID", "id"),
            ("SoundKitID", 0),
            ("FileDataID", ("int", 1, 0)),
            ("Frequency", ("u8", 2)),
            ("Volume", ("float", 3, 0)),
            ("PlayerConditionID", ("int", 4, 0)),
        ],
    ),
    # WoWDBDefs layout CAE394E7: water/ocean alphas are fields 6-9.
    "LightParams": (
        0xCAE394E7,
        [
            ("ID", "id"),
            ("WaterShallowAlpha", ("float", 6, 0)),
            ("WaterDeepAlpha", ("float", 7, 0)),
            ("OceanShallowAlpha", ("float", 8, 0)),
            ("OceanDeepAlpha", ("float", 9, 0)),
        ],
    ),
    # WoWDBDefs layout D1ECEEC9. WebWowViewerCpp reads Color[0..1], Float[0..17], Int[0..3] and
    # Coefficient[0..3].
    "LiquidType": (
        0xD1ECEEC9,
        [("ID", "id"), ("Name", ("string", 0)), ("Flags", ("int", 2, 0)), ("MaterialID", ("u8", 14))]
        + [(f"FrameCountTexture_{i}", ("u8", 16, i)) for i in range(6)]
        + [(f"Color_{i}", ("int", 17, i)) for i in range(3)]
        + [(f"Float_{i}", ("float", 18, i)) for i in range(18)]
        + [(f"Int_{i}", ("int", 19, i)) for i in range(4)]
        + [(f"Coefficient_{i}", ("float", 20, i)) for i in range(4)],
    ),
    "LiquidMaterial": (0x98E5D7AA, [("ID", "id"), ("Flags", ("int", 0, 0)), ("LVF", ("u8", 1))]),
    "LiquidObject": (
        0xCB0D39E8,
        [
            ("ID", "id"),
            ("FlowDirection", ("float", 0, 0)),
            ("FlowSpeed", ("float", 1, 0)),
            ("LiquidTypeID", ("u16", 2)),
        ],
    ),
    # Type: -1 file texture, 0/1/2 procedural ocean/river/WMO depth texture.
    "LiquidTypeXTexture": (
        0x7BEECC7F,
        [
            ("ID", "id"),
            ("FileDataID", ("int", 0, 0)),
            ("OrderIndex", ("int", 1, 0)),
            ("Type", ("i8", 2)),
            ("LiquidTypeID", "parent"),
        ],
    ),
}

# Narrow DBD types: pallet entries are 32-bit and carry unrelated high bits.
NARROW = {"i8": (True, 8), "u8": (False, 8), "i16": (True, 16), "u16": (False, 16)}

# Tables whose inline ID is not their first field.
INLINE_ID_FIELD = {"SpellVisualMissile": 2, "ChrModel": 2}


def read_fields(data, field_count, sections):
    start = 204 + sections * 40 + field_count * 4
    fields = [struct.unpack_from("<HH5I", data, start + i * 24) for i in range(field_count)]
    # Pallet (3, 4) and common (2) fields each index their own block, in field order.
    palette_offsets, offset = [], 0
    for field in fields:
        palette_offsets.append(offset)
        offset += field[2] if field[3] in (3, 4) else 0
    return fields, palette_offsets, start + field_count * 24


def read_common(data, fields, common_start):
    """Per field, the {record id: value} of a common-data (storage 2) field; other fields None."""
    common, offset = [], common_start
    for field in fields:
        if field[3] != 2:
            common.append(None)
            continue
        common.append(dict(struct.iter_unpack("<II", data[offset : offset + field[2]])))
        offset += field[2]
    return common


def decode_field(raw, field, palette, palette_offset):
    bit_offset, width, _, storage, _, _, array_count = field
    value = (raw >> bit_offset) & ((1 << width) - 1)
    if storage == 3:
        return struct.unpack_from("<I", palette, palette_offset + value * 4)[0]
    if storage == 4:
        return struct.unpack_from(f"<{array_count}I", palette, palette_offset + value * 4 * array_count)
    if storage == 5 and width and value & (1 << (width - 1)):
        return value - (1 << width)
    if storage == 2:
        return None  # resolved per record id from the common block
    if storage not in (0, 1, 5):
        raise ValueError(f"unsupported field storage {storage}")
    return value


def read_relations(data, offset, size):
    if not size:
        return {}
    entries = struct.unpack_from("<I", data, offset)[0]
    return {index: parent for parent, index in struct.iter_unpack("<II", data[offset + 12 : offset + 12 + entries * 8])}


def read_wdc5(data, layout, id_field=0):
    if data[:4] != b"WDC5":
        raise ValueError("not a WDC5 file")
    _, field_count, record_size, _, _, actual_layout = struct.unpack_from("<6I", data, 136)
    flags, _, _, _, _, _, common_size, palette_size, sections = struct.unpack_from("<HH7I", data, 172)
    if actual_layout != layout:
        raise ValueError(f"layout {actual_layout:08X}, expected {layout:08X}")
    if flags & ~0x4:
        raise ValueError(f"unsupported WDC5 flags {flags:#x}")
    fields, palette_offsets, palette_start = read_fields(data, field_count, sections)
    palette = data[palette_start : palette_start + palette_size]
    common = read_common(data, fields, palette_start + palette_size)
    if sum(f[2] for f in fields if f[3] == 2) != common_size:
        raise ValueError("common data size mismatch")
    rows, dropped = {}, 0
    for section in range(sections):
        key, start, count, string_size, _, id_size, relation_size, _, copies = struct.unpack_from(
            "<Q8I", data, 204 + section * 40
        )
        payload = data[start : start + count * record_size]
        if key and count and not any(payload):
            dropped += count
            continue
        id_start = start + count * record_size + string_size
        copy_start = id_start + id_size
        relations = read_relations(data, copy_start + copies * 8, relation_size)
        for i in range(count):
            raw = int.from_bytes(payload[i * record_size : (i + 1) * record_size], "little")
            values = [decode_field(raw, f, palette, o) for f, o in zip(fields, palette_offsets)]
            row_id = struct.unpack_from("<I", data, id_start + i * 4)[0] if id_size else values[id_field]
            for index, defaults in enumerate(common):
                if defaults is not None:
                    values[index] = defaults.get(row_id, fields[index][4])
            rows[row_id] = (values, relations.get(i), start + i * record_size)
        for new_id, source in struct.iter_unpack("<II", data[copy_start : copy_start + copies * 8]):
            rows[new_id] = rows[source]
    return rows, dropped, fields, sections


def read_string(data, record_offset, field, value):
    """Inline string fields hold the string's offset from the field's own byte position."""
    start = record_offset + field[0] // 8 + value
    return data[start : data.index(b"\0", start)].decode("utf-8")


def main():
    table, db2_path, out_path = sys.argv[1:4]
    layout, columns = TABLES[table]
    data = open(db2_path, "rb").read()
    rows, dropped, fields, sections = read_wdc5(data, layout, INLINE_ID_FIELD.get(table, 0))
    if sections != 1 and any(isinstance(s, tuple) and s[0] == "string" for _, s in columns):
        raise ValueError(f"string columns need a single-section table, got {sections} sections")
    with open(out_path, "w", newline="") as handle:
        out = csv.writer(handle)
        out.writerow([name for name, _ in columns])
        for row_id in sorted(rows):
            values, parent, record_offset = rows[row_id]

            def column(s):
                if isinstance(s, tuple) and s[0] in NARROW:
                    signed, width = NARROW[s[0]]
                    value = values[s[1]][s[2]] if len(s) == 3 else values[s[1]]
                    value &= (1 << width) - 1
                    return value - (1 << width) if signed and value >> (width - 1) else value
                if isinstance(s, tuple) and s[0] in ("float", "int"):
                    value = values[s[1]]
                    bits = value[s[2]] if isinstance(value, tuple) else (value >> (32 * s[2])) & 0xFFFFFFFF
                    bits &= 0xFFFFFFFF
                    if s[0] == "int":
                        return struct.unpack("<i", struct.pack("<I", bits))[0]
                    return "%.9g" % struct.unpack("<f", struct.pack("<I", bits))[0]
                if isinstance(s, tuple):
                    return read_string(data, record_offset, fields[s[1]], values[s[1]])
                return {"id": row_id, "parent": parent}[s] if isinstance(s, str) else values[s]

            out.writerow([column(s) for _, s in columns])
    print(f"{table}: {len(rows)} rows, {dropped} encrypted records dropped", file=sys.stderr)


if __name__ == "__main__":
    main()
