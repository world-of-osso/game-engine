"""Spell visual asset joins matching godot/core/src/spell_visual.rs.

Enumerate every authored variant for the selected spells, not one conditional
cast. Unsupported kit effects remain explicit unresolved edges.
"""
from pathlib import PurePosixPath


def value(row, key):
    return int(row.get(key) or 0)


def seed_sound_kits(graph, catalogs, kits):
    found = set()
    for row in catalogs.rows('SoundKitEntry'):
        kit = value(row, 'SoundKitID')
        if kit not in kits:
            continue
        found.add(kit)
        fdid = value(row, 'FileDataID')
        if not fdid:
            continue
        kind = PurePosixPath(graph.paths.get(fdid, '')).suffix.lstrip('.')
        if kind not in {'ogg', 'mp3', 'wav'}:
            graph.issue('unknown_sound_file_type', f'SoundKit {kit} file {fdid}', fdid)
            continue
        graph.add(fdid, kind, f'SoundKit {kit} entry {row["ID"]}')
    for kit in sorted(kits - found):
        graph.issue('missing_metadata_row', f'SoundKitEntry SoundKitID={kit}')


def expand_kit_ids(catalogs, kits):
    rows = catalogs.index('SpellVisualKit')
    pending = list(kits)
    while pending:
        kit = pending.pop()
        other = value(rows.get(kit, {}), 'FallbackSpellVisualKitID')
        if other and other not in kits:
            kits.add(other)
            pending.append(other)
    return kits


def seed_spell_visuals(graph, catalogs, spells):
    wanted = set(spells)
    visuals = set()
    for row in catalogs.rows('SpellMisc'):
        spell = value(row, 'SpellID')
        if spell in wanted:
            for field in ['SpellIconFileDataID', 'ActiveIconFileDataID']:
                graph.add(value(row, field), 'blp', f'spell {spell} difficulty {row.get("DifficultyID", "")} {field}')
    for row in catalogs.rows('SpellXSpellVisual'):
        spell = value(row, 'SpellID')
        if spell not in wanted:
            continue
        visuals.add(value(row, 'SpellVisualID'))
        for field in ['SpellIconFileID', 'ActiveIconFileID']:
            graph.add(value(row, field), 'blp', f'spell {spell} {field}')
    visuals.discard(0)
    kits = {value(row, 'SpellVisualKitID') for row in catalogs.rows('SpellVisualEvent')
            if value(row, 'SpellVisualID') in visuals} - {0}
    kits = expand_kit_ids(catalogs, kits)
    effects = set()
    sounds = set()
    attachments = catalogs.index('SpellVisualKitModelAttach')
    for row in attachments.values():
        if value(row, 'ParentSpellVisualKitID') in kits:
            effects.add(value(row, 'SpellVisualEffectNameID'))
    for row in catalogs.rows('SpellVisualKitEffect'):
        if value(row, 'ParentSpellVisualKitID') not in kits:
            continue
        kind, effect = value(row, 'EffectType'), value(row, 'Effect')
        if kind == 2:
            attachment = attachments.get(effect)
            if attachment is None:
                graph.issue('missing_metadata_row', f'SpellVisualKitModelAttach ID={effect}')
            else:
                effects.add(value(attachment, 'SpellVisualEffectNameID'))
        elif kind == 5:
            sounds.add(effect)
        elif kind != 6:
            graph.issue('unsupported_spell_kit_effect', f'kit {row["ParentSpellVisualKitID"]} effect type {kind} effect {effect}')
    missiles = {value(row, 'SpellVisualMissileSetID') for row in catalogs.rows('SpellVisual')
                if value(row, 'ID') in visuals} - {0}
    for row in catalogs.rows('SpellVisualMissile'):
        if value(row, 'SpellVisualMissileSetID') in missiles:
            effects.add(value(row, 'SpellVisualEffectNameID'))
            sounds.add(value(row, 'SoundEntriesID'))
    names = catalogs.index('SpellVisualEffectName')
    for effect in sorted(effects - {0}):
        row = names.get(effect)
        if row is None:
            graph.issue('missing_metadata_row', f'SpellVisualEffectName ID={effect}')
            continue
        model = value(row, 'ModelFileDataID')
        if model > 0:
            graph.add(model, 'm2', f'SpellVisualEffectName {effect} model')
        elif model < 0:
            graph.issue('unsupported_spell_procedural_model', f'SpellVisualEffectName {effect} model {model}')
        graph.add(value(row, 'TextureFileDataID'), 'blp', f'SpellVisualEffectName {effect} texture')
    seed_sound_kits(graph, catalogs, sounds - {0})
