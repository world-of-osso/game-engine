"""Spell visual asset joins matching godot/core/src/spell_visual.rs.

Enumerate every authored variant for the selected spells, not one conditional
cast. Unsupported kit effects remain explicit unresolved edges.
"""

from pathlib import PurePosixPath


def value(row, key):
    return int(row.get(key) or 0)


def seed_sound_kits(graph, catalogs, kits):
    found = set()
    for row in catalogs.rows("SoundKitEntry"):
        kit = value(row, "SoundKitID")
        if kit not in kits:
            continue
        found.add(kit)
        fdid = value(row, "FileDataID")
        if not fdid:
            continue
        kind = PurePosixPath(graph.paths.get(fdid, "")).suffix.lstrip(".")
        numeric_audio = graph.data / f"sounds/{fdid}.audio"
        if kind not in {"ogg", "mp3", "wav"}:
            kind = identify_local_sound(numeric_audio)
            if kind is None:
                graph.issue(
                    "unknown_sound_file_type", f"SoundKit {kit} file {fdid}", fdid
                )
                kind = "audio"
            else:
                graph.resolve(
                    "sound_file_type",
                    fdid,
                    "resolved",
                    f"local extracted header identifies {kind}; no listfile extension invented",
                )
        alias = f"sounds/spells/{fdid}.ogg" if kind in {"ogg", "audio"} else None
        graph.add(fdid, kind, f"SoundKit {kit} entry {row['ID']}", alias=alias)
        if kind != "audio" and numeric_audio.is_file():
            graph.add(
                fdid,
                kind,
                f"SoundKit {kit} extracted header",
                alias=f"sounds/{fdid}.audio",
            )
    for kit in sorted(kits - found):
        graph.issue("missing_metadata_row", f"SoundKitEntry SoundKitID={kit}")


def identify_local_sound(path):
    if not path.is_file():
        return None
    with path.open("rb") as stream:
        header = stream.read(12)
    if header.startswith(b"OggS"):
        return "ogg"
    if header.startswith(b"ID3"):
        return "mp3"
    if header[:4] == b"RIFF" and header[8:12] == b"WAVE":
        return "wav"
    if len(header) >= 4 and header[0] == 0xFF and header[1] & 0xE0 == 0xE0:
        # MPEG audio frame: reject reserved version/layer/rate/sample-rate bits.
        if (
            header[1] & 0x18 != 0x08
            and header[1] & 0x06
            and header[2] >> 4 not in {0, 15}
            and header[2] & 0x0C != 0x0C
        ):
            return "mp3"
    return None


def expand_kit_ids(catalogs, kits):
    rows = catalogs.index("SpellVisualKit")
    pending = list(kits)
    while pending:
        kit = pending.pop()
        other = value(rows.get(kit, {}), "FallbackSpellVisualKitID")
        if other and other not in kits:
            kits.add(other)
            pending.append(other)
    return kits


def seed_spell_visuals(graph, catalogs, spells):
    wanted = set(spells)
    visuals = set()
    for row in catalogs.rows("SpellMisc"):
        spell = value(row, "SpellID")
        if spell in wanted:
            for field in ["SpellIconFileDataID", "ActiveIconFileDataID"]:
                graph.add(
                    value(row, field),
                    "blp",
                    f"spell {spell} difficulty {row.get('DifficultyID', '')} {field}",
                )
    for row in catalogs.rows("SpellXSpellVisual"):
        spell = value(row, "SpellID")
        if spell not in wanted:
            continue
        visuals.add(value(row, "SpellVisualID"))
        for field in ["SpellIconFileID", "ActiveIconFileID"]:
            graph.add(value(row, field), "blp", f"spell {spell} {field}")
    visuals.discard(0)
    kits = {
        value(row, "SpellVisualKitID")
        for row in catalogs.rows("SpellVisualEvent")
        if value(row, "SpellVisualID") in visuals
    } - {0}
    kits = expand_kit_ids(catalogs, kits)
    effects = set()
    sounds = set()
    attachments = catalogs.index("SpellVisualKitModelAttach")
    from closure_kit_effects import KitEffectReferences
    from collections import defaultdict

    attach_names = defaultdict(set)
    for row in attachments.values():
        attach_names[value(row, "ParentSpellVisualKitID")].add(
            value(row, "SpellVisualEffectNameID")
        )
    kit_rows = defaultdict(list)
    for row in catalogs.rows("SpellVisualKitEffect"):
        kit_rows[value(row, "ParentSpellVisualKitID")].append(row)
    references = KitEffectReferences(graph, catalogs, effects, sounds, kits)
    visited = set()
    while kits - visited:
        expand_kit_ids(catalogs, kits)
        for kit in sorted(kits - visited):
            visited.add(kit)
            effects.update(attach_names[kit])
            for row in kit_rows[kit]:
                kind, effect = value(row, "EffectType"), value(row, "Effect")
                references.seed(
                    kind, effect, f"kit {kit} effect type {kind} effect {effect}"
                )
    missiles = {
        value(row, "SpellVisualMissileSetID")
        for row in catalogs.rows("SpellVisual")
        if value(row, "ID") in visuals
    } - {0}
    for row in catalogs.rows("SpellVisualMissile"):
        if value(row, "SpellVisualMissileSetID") in missiles:
            effects.add(value(row, "SpellVisualEffectNameID"))
            sounds.add(value(row, "SoundEntriesID"))
    names = catalogs.index("SpellVisualEffectName")
    for effect in sorted(effects - {0}):
        row = names.get(effect)
        if row is None:
            graph.issue("missing_metadata_row", f"SpellVisualEffectName ID={effect}")
            continue
        model = value(row, "ModelFileDataID")
        if model > 0:
            graph.add(model, "m2", f"SpellVisualEffectName {effect} model")
        elif model < 0:
            graph.issue(
                "unsupported_spell_procedural_model",
                f"SpellVisualEffectName {effect} model {model}",
            )
        graph.add(
            value(row, "TextureFileDataID"),
            "blp",
            f"SpellVisualEffectName {effect} texture",
        )
    seed_sound_kits(graph, catalogs, sounds - {0})
