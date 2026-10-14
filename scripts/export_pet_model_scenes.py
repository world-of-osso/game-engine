#!/usr/bin/env python3
"""Export authored Retail pet-card/loadout cameras and actor transforms from local DB2.

Extract FDIDs 1548213-1548216 with casc-local from the installed Retail build.
Pass WoWDBDefs definitions whose layout hashes match those bytes. No CDN or
synthetic framing values. BattlePetSpecies CardUIModelSceneID/LoadoutUIModelSceneID
select the exported scene. XML continues to define viewport dimensions.
"""
import argparse
import csv
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import struct

from adventure_guide_import import TABLES, read_table

SOURCES = {'UiModelSceneCamera': 1548213, 'UiModelSceneActor': 1548214,
           'UiModelScene': 1548215, 'UiModelSceneActorDisplay': 1548216}


def group_by_scene(rows):
    groups = defaultdict(list)
    for row in rows:
        groups[row['UiModelSceneID']].append(row)
    return groups


def single_tagged(rows, tag, scene, kind):
    matches = [row for row in rows if row['ScriptTag'] == tag]
    if len(matches) != 1:
        raise ValueError(f'scene {scene}: expected one {kind} tagged {tag}, found {len(matches)}')
    return matches[0]


def export_scene(scene, cameras, actors, displays):
    scene_id = scene['ID']
    camera = single_tagged(cameras[scene_id], 'primary', scene_id, 'camera')
    tag = 'unwrapped' if scene['UiSystemType'] == 1 else 'pet'
    actor = single_tagged(actors[scene_id], tag, scene_id, 'actor')
    display_id = actor['UiModelSceneActorDisplayID']
    if display_id not in displays:
        raise ValueError(f'scene {scene_id}: actor display {display_id} missing')
    if camera['CameraType'] != 0:
        raise ValueError(f'scene {scene_id}: unsupported camera type {camera["CameraType"]}')
    offsets = [value for key, value in camera.items() if key.startswith('Zoomed')]
    if any(offsets):
        raise ValueError(f'scene {scene_id}: zoom spline needs explicit export support')
    return {'id': scene_id, 'target': [camera[f'Target_{axis}'] for axis in range(3)],
            'yaw': camera['Yaw'], 'pitch': camera['Pitch'], 'roll': camera['Roll'],
            'zoom_distance': camera['ZoomDistance'],
            'actor_position': [actor[f'Position_{axis}'] for axis in range(3)],
            'actor_orientation': [actor[f'Orientation{axis}'] for axis in ['Yaw', 'Pitch', 'Roll']],
            'origin_flags': actor['Flags'], 'normalized_scale': actor['NormalizedScale'],
            'scale': displays[display_id]['Scale']}


def build_scenes(tables, scene_ids):
    cameras = group_by_scene(tables['UiModelSceneCamera'])
    actors = group_by_scene(tables['UiModelSceneActor'])
    displays = {row['ID']: row for row in tables['UiModelSceneActorDisplay']}
    scenes = {row['ID']: row for row in tables['UiModelScene']}
    missing = scene_ids - scenes.keys()
    if missing:
        raise ValueError(f'BattlePetSpecies references missing scenes {sorted(missing)}')
    return [export_scene(scenes[scene_id], cameras, actors, displays) for scene_id in sorted(scene_ids)]


def read_sources(directory, definitions):
    tables, receipts = {}, []
    for name, fdid in SOURCES.items():
        path = directory / f'{fdid}.db2'
        raw = path.read_bytes()
        layout = struct.unpack_from('<I', raw, 156)[0]
        definition = definitions / f'{name}.dbd'
        TABLES[name] = (fdid, layout)
        columns, rows, dropped = read_table(name, path, definition)
        if dropped:
            raise ValueError(f'{name}: {dropped} unreadable records')
        tables[name] = [dict(zip([column for column, _ in columns], row)) for row in rows]
        receipts.append({'table': name, 'fdid': fdid, 'layout': f'{layout:08X}', 'records': len(rows),
                         'sha256': hashlib.sha256(raw).hexdigest(),
                         'schema_sha256': hashlib.sha256(definition.read_bytes()).hexdigest()})
    return tables, receipts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db2', type=Path, required=True)
    parser.add_argument('--definitions', type=Path, required=True)
    parser.add_argument('--build', required=True)
    parser.add_argument('--species', type=Path, required=True, help='authored BattlePetSpecies.csv used by runtime')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    tables, receipts = read_sources(args.db2, args.definitions)
    with args.species.open(newline='') as handle:
        scene_ids = {int(row[field]) for row in csv.DictReader(handle)
                     for field in ['CardUIModelSceneID', 'LoadoutUIModelSceneID']}
    scenes = build_scenes(tables, scene_ids)
    args.output.write_text(json.dumps(scenes, indent=2) + '\n')
    args.output.with_suffix('.provenance.json').write_text(json.dumps(
        {'build': args.build, 'tables': receipts, 'scenes': len(scenes),
         'species_sha256': hashlib.sha256(args.species.read_bytes()).hexdigest()}, indent=2) + '\n')
    print(f'Exported {len(scenes)} authored pet model scenes to {args.output}')


if __name__ == '__main__':
    main()
