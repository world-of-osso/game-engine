"""Decode authenticated local DB2 payloads using their exact DBD layout.

Fixed-record decoding uses the engine WDC5 reader. Sparse decoding uses the
server's existing WDC5 reader, supplied explicitly by the refresh command.
"""
import csv
import re
import struct
from pathlib import Path

# Stable existing CSV names. These rename DBD symbols, never choose binary indexes.
CSV_NAMES = {
    'CooldownSet': {'ChrSpecialization': 'ChrSpecializationID'},
    'Achievement': {'Criteria_tree': 'CriteriaTree'},
    'Criteria': {'Modifier_tree_ID': 'ModifierTreeId', 'Start_event': 'StartEvent',
                 'Start_asset': 'StartAsset', 'Start_timer': 'StartTimer',
                 'Fail_event': 'FailEvent', 'Fail_asset': 'FailAsset',
                 'Eligibility_world_state_ID': 'EligibilityWorldStateID',
                 'Eligibility_world_state_value': 'EligibilityWorldStateValue'},
    'JournalInstanceEntrance': {'Position': 'Pos'},
    'WarbandScenePlacementOption': {'ID': 'OptionID', 'Field_11_1_0_58221_005': 'LayoutKey'},
    'TraitTreeXTraitCurrency': {'Index': '_Index'},
    'TraitNodeGroupXTraitNode': {'Index': '_Index'},
    'TraitNodeXTraitNodeEntry': {'Index': '_Index'},
}


def layout_fields(definition, layout):
    text = Path(definition).read_text()
    types = {}
    for line in text.splitlines()[1:]:
        if line.startswith(('LAYOUT', 'BUILD')):
            break
        match = re.match(r'(int|float|string|locstring)(?:<[^>]*>)?\s+(\w+)', line)
        if match:
            types[match[2]] = match[1]
    lines = text.splitlines()
    start = next((i for i, line in enumerate(lines)
                  if line.startswith('LAYOUT ') and f'{layout:08X}' in line.split(' ', 1)[1].split(', ')), None)
    if start is None:
        raise ValueError(f'{definition}: layout {layout:08X} absent')
    fields = []
    for line in lines[start + 1:]:
        if line.startswith('LAYOUT') or (line.startswith('BUILD') and fields):
            break
        line = line.split('//', 1)[0].strip()
        if not line or line.startswith(('BUILD', 'COMMENT')):
            continue
        match = re.fullmatch(r'(?:\$([^$]+)\$)?(\w+)(?:<([^>]+)>)?(?:\[(\d+)\])?', line)
        if match is None:
            raise ValueError(f'{definition}: unsupported DBD field {line}')
        tags, name, width, count = match.groups()
        kind = types[name]
        name = CSV_NAMES.get(Path(definition).stem, {}).get(name, name)
        fields.append((name + '_lang' if kind == 'locstring' and not name.endswith('_lang') else name,
                       kind, (tags or '').split(','), width, int(count or 1)))
    if not fields:
        raise ValueError(f'{definition}: empty layout {layout:08X}')
    return fields


def columns(fields):
    return [name if count == 1 else f'{name}_{i}'
            for name, _, _, _, count in fields for i in range(count)]


def number(raw, kind, width, element=0):
    bits = int((width or '32').lstrip('u'))
    value = raw[element] if isinstance(raw, tuple) else raw >> (bits * element)
    value &= (1 << bits) - 1
    if kind == 'float':
        return format(struct.unpack('<f', struct.pack('<I', value))[0], '.9g')
    if width and not width.startswith('u') and value & (1 << (bits - 1)):
        value -= 1 << bits
    return value


def fixed_rows(data, fields, layout, reader):
    inline = [f for f in fields if 'noninline' not in f[2]]
    id_field = next((i for i, f in enumerate(inline) if 'id' in f[2]), 0)
    records, dropped, descriptors, _ = reader.read_wdc5(data, layout, id_field)
    rows = []
    for row_id, (values, parent, offset) in sorted(records.items()):
        row, index = [], 0
        for name, kind, tags, width, count in fields:
            if 'noninline' in tags:
                if 'id' in tags:
                    row.append(row_id)
                elif 'relation' in tags:
                    row.append(parent if parent is not None else 0)
                else:
                    raise ValueError(f'unsupported noninline field {name}')
                continue
            if 'id' in tags:
                row.append(row_id)
            elif kind in ('string', 'locstring'):
                for element in range(count):
                    value = number(values[index], 'int', 'u32', element)
                    descriptor = (descriptors[index][0] + element * 32, *descriptors[index][1:])
                    row.append(reader.read_string(data, offset, descriptor, value))
            else:
                row.extend(number(values[index], kind, width, i) for i in range(count))
            index += 1
        rows.append(row)
    return rows, dropped


def sparse_rows(data, fields, reader):
    if reader is None:
        raise ValueError('sparse DB2 requires --server-scripts (existing server WDC5 decoder)')
    inline = [f for f in fields if 'noninline' not in f[2]]
    noninline_id = any('noninline' in f[2] and 'id' in f[2] for f in fields)
    strings = {i + int(noninline_id) for i, f in enumerate(inline) if f[1] in ('string', 'locstring')}
    arrays = {i: f[4] for i, f in enumerate(inline) if f[4] > 1}
    types = {i: 'REAL' if f[1] == 'float' else 'UNSIGNED' if (f[3] or '').startswith('u') else 'INTEGER'
             for i, f in enumerate(inline) if f[1] not in ('string', 'locstring')}
    header, records = reader.decode(data, strings, inline_arrays=arrays, field_types=types)
    rows = [records[k] for k in sorted(records)]
    expected = len(columns(fields))
    if any(len(row) != expected for row in rows):
        raise ValueError(f'sparse schema column count differs from decoded row ({expected})')
    dropped = len(header.get('unreadable_record_ids', []))
    for section in range(header['section_count']):
        key, start, count, _, end, *_ = struct.unpack_from('<Q8I', data, 204 + section * 40)
        if key and count and not any(data[start:end]):
            dropped += count
    return rows, dropped


def export(data, definition, output, fixed_reader, sparse_reader=None):
    if data[:4] != b'WDC5':
        raise ValueError('expected WDC5 payload')
    layout = struct.unpack_from('<I', data, 156)[0]
    fields = layout_fields(definition, layout)
    flags = struct.unpack_from('<H', data, 172)[0]
    rows, dropped = (sparse_rows(data, fields, sparse_reader) if flags & 1
                     else fixed_rows(data, fields, layout, fixed_reader))
    names = columns(fields)
    if Path(definition).stem == 'DungeonEncounter':
        # Existing server CSV contract exposes the relationship column separately.
        # DBD marks MapID as the inline relation; it is not a second binary field.
        relation = next(i for i, f in enumerate(fields) if 'relation' in f[2])
        names.append('RelationMapID')
        rows = [[*row, row[relation]] for row in rows]
    with Path(output).open('w', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow(names)
        writer.writerows(rows)
    id_column = next(name for name, _, tags, _, _ in fields if 'id' in tags)
    return {'layout': f'{layout:08X}', 'rows': len(rows), 'unreadable_records': max(0, dropped),
            'id_column': id_column}
