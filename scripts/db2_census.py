#!/usr/bin/env python3
"""Row/schema census of two DB2 snapshots; read-only TDB joins and code candidates."""
import argparse
import csv
import json
import math
from pathlib import Path
import re
import sqlite3


def read_rows(path, id_column='ID'):
    with path.open(newline='', encoding='utf-8-sig') as stream:
        return {row[id_column]: row for row in csv.DictReader(stream)}


def equal_cell(column, left, right):
    if left == right:
        return True
    if any(word in column.lower() for word in ('name', 'lang', 'description', 'directory')):
        return False
    if re.fullmatch(r'-?\d+', left) and re.fullmatch(r'-?\d+', right):
        return int(left) == int(right)
    try:
        return math.isclose(float(left), float(right), rel_tol=1e-7, abs_tol=1e-10)
    except ValueError:
        return False


def compare_rows(old, new):
    old_columns = set(next(iter(old.values()), {}))
    new_columns = set(next(iter(new.values()), {}))
    common = old_columns & new_columns
    changed = {}
    for key in old.keys() & new.keys():
        fields = sorted(c for c in common if not equal_cell(c, old[key][c], new[key][c]))
        if fields:
            changed[key] = fields
    identity = {'Name', 'Name_lang', 'Title_lang', 'Directory', 'ModelID', 'FileDataID',
                'SpellID', 'CreatureDisplayInfoID', 'ItemID', 'Display_lang'}
    return {'old_rows': len(old), 'new_rows': len(new),
            'added_ids': sorted(map(int, new.keys() - old.keys())),
            'removed_ids': sorted(map(int, old.keys() - new.keys())),
            'changed_ids': sorted(map(int, changed)), 'changed_fields': changed,
            'identity_changed_ids': sorted(int(k) for k, fields in changed.items() if identity.intersection(fields)),
            'added_columns': sorted(new_columns - old_columns),
            'removed_columns': sorted(old_columns - new_columns)}


def source_candidates(repos, tables):
    changes = {t: (set(d.get('removed_ids', [])), set(d.get('identity_changed_ids', [])))
               for t, d in tables.items()}
    result = []
    for repo in repos:
        for root in ('scripts', 'godot', 'tools', 'src', 'crates', 'tests'):
            for path in (repo / root).rglob('*'):
                if path.suffix not in ('.rs', '.py', '.gd', '.js', '.sql', '.json') or 'target' in path.parts:
                    continue
                text = path.read_text()
                names = set(re.findall(r'[A-Za-z][A-Za-z0-9]+', text)) & changes.keys()
                numbers = {int(v) for v in re.findall(r'(?<![A-Za-z0-9])([0-9]{1,9})(?![A-Za-z0-9])', text)}
                for table in sorted(names):
                    removed, changed = changes[table]
                    missing = sorted(numbers & removed)
                    meaning = sorted(numbers & changed)
                    if missing or meaning:
                        result.append({'path': str(path.relative_to(repo)), 'repo': repo.name, 'table': table,
                                       'removed_id_candidates': missing, 'identity_changed_id_candidates': meaning})
    return result


# Explicit TDB column semantics. No name-only inference silently promotes a join.
WORLD_JOINS = {
    'creature_template_model': {'CreatureDisplayID': 'CreatureDisplayInfo'},
    'creature_template': {'family': 'CreatureFamily', 'faction': 'FactionTemplate'},
    'item_template': {'entry': 'Item', 'displayid': 'ItemDisplayInfo'},
    'quest_template': {'ID': 'QuestV2', 'RewardSpell': 'Spell', 'RewardSpellCast': 'Spell'},
    'npc_spellclick_spells': {'spell_id': 'Spell'},
    'creature_equip_template': {'ItemID1': 'Item', 'ItemID2': 'Item', 'ItemID3': 'Item'},
    'gameobject_template': {'displayId': 'GameObjectDisplayInfo'},
    'spell_script_names': {'spell_id': 'Spell'},
    'spell_area': {'spell': 'Spell', 'area': 'AreaTable'},
    'creature': {'map': 'Map'}, 'gameobject': {'map': 'Map'},
    'playercreateinfo': {'map': 'Map', 'zone': 'AreaTable'},
}


def world_joins(world, new, differences):
    conn = sqlite3.connect(f'file:{world}?mode=ro', uri=True)
    available = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    result = []
    for table, joins in WORLD_JOINS.items():
        if table not in available:
            continue
        columns = {r[1] for r in conn.execute(f'PRAGMA table_info("{table}")')}
        for column, source in joins.items():
            if column not in columns or not (new / f'{source}.csv').exists():
                continue
            ids = {int(r[0]) for r in conn.execute(f'SELECT DISTINCT "{column}" FROM "{table}" WHERE "{column}" > 0')}
            rows = read_rows(new / f'{source}.csv')
            missing = sorted(ids - set(map(int, rows)))
            changed = sorted(ids & set(differences.get(source, {}).get('identity_changed_ids', [])))
            result.append({'table': table, 'column': column, 'db2_table': source, 'referenced_ids': len(ids),
                           'missing_in_new': missing, 'newly_missing': sorted(set(missing) & set(differences.get(source, {}).get('removed_ids', []))),
                           'identity_changed': changed})
    conn.close()
    return result


def census(args):
    receipt = json.loads((args.new / 'receipt.json').read_text())
    tables = {}
    with (args.output / 'tables.csv').open('w', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow(['table', 'old_rows', 'new_rows', 'adds', 'removes', 'changes', 'identity_changes', 'schema_added', 'schema_removed'])
        for path in sorted(args.new.glob('*.csv')):
            previous = args.old / path.name
            if not previous.exists():
                continue
            id_column = receipt['tables'][path.stem]['id_column']
            diff = compare_rows(read_rows(previous, id_column), read_rows(path, id_column))
            tables[path.stem] = diff
            writer.writerow([path.stem, diff['old_rows'], diff['new_rows'], len(diff['added_ids']), len(diff['removed_ids']),
                             len(diff['changed_ids']), len(diff['identity_changed_ids']), '|'.join(diff['added_columns']), '|'.join(diff['removed_columns'])])
    document = {'source': receipt['identity'], 'old': str(args.old), 'tables': tables,
                'unavailable_sources': receipt['unavailable'],
                'code_candidates_note': 'Numeric literals in files naming the same table are candidates, not proof of a typed reference. Identity changes compare names and referenced object/model/spell IDs; other field changes are not called changed meaning.',
                'code_candidates': source_candidates([args.engine, args.server], tables),
                'world_join_scope': WORLD_JOINS,
                'world_joins': world_joins(args.world, args.new, tables)}
    (args.output / 'census.json').write_text(json.dumps(document, indent=2) + '\n')
    summary = {'compared_tables': len(tables),
               'added_rows': sum(len(d['added_ids']) for d in tables.values()),
               'removed_rows': sum(len(d['removed_ids']) for d in tables.values()),
               'changed_rows': sum(len(d['changed_ids']) for d in tables.values()),
               'identity_changed_rows': sum(len(d['identity_changed_ids']) for d in tables.values()),
               'world_newly_missing_ids': sum(len(j['newly_missing']) for j in document['world_joins']),
               'code_candidate_groups': len(document['code_candidates'])}
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('old', 'new', 'output', 'engine', 'server', 'world'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    census(args)


if __name__ == '__main__':
    main()
