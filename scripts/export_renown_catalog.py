#!/usr/bin/env python3
"""Export the Retail 12.1.0.69933 renown catalog for the realm's Journeys service.

Major factions are Covenant rows whose Faction has a RenownCurrencyID (TrinityCore
a352b1fa ReputationMgr::IsRenownReputation). Renown level is that currency, capped at
CurrencyTypes.MaxQty; the per-level threshold is Faction.ReputationMax. RenownRewards
rows are keyed by CovenantID. Extract the FDIDs below with casc-local first and pass the
pinned WoWDBDefs .dbd files; unreadable records are counted, never synthesized.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from db2_source import source_identity

from adventure_guide_import import TABLES, read_table

SOURCES = {
    'Covenant': (3384973, 0xB8563783),
    'Faction': (1361972, 0xE7553DC7),
    'CurrencyTypes': (1095531, 0xFA3226AB),
    'RenownRewards': (3743117, 0x30AFBCF8),
    'PlayerCondition': (1045411, 0x1D743D99),
}
REWARD_FIELDS = {
    'ID': 'id', 'Level': 'level', 'UiOrder': 'ui_order', 'Flags': 'flags', 'Icon': 'icon',
    'Name_lang': 'name', 'Description_lang': 'description', 'ToastDescription_lang': 'toast',
    'ItemID': 'item_id', 'SpellID': 'spell_id', 'MountID': 'mount_id', 'TransmogID': 'transmog_id',
    'TransmogSetID': 'transmog_set_id', 'TransmogIllusionID': 'transmog_illusion_id',
    'CharTitlesID': 'title_id', 'QuestID': 'quest_id', 'PlayerConditionID': 'player_condition_id',
}


def level_threshold(faction):
    """ReputationMax for TC GetFactionDataIndexForRaceAndClass; only universal index 0 is modeled."""
    universal = (faction['ReputationClassMask_0'] == -1 and faction['ReputationRaceMasks0_0'] == -1
                 and faction['ReputationRaceMasks0_1'] == -1)
    if not universal or faction['ReputationMax_0'] <= 0:
        raise ValueError(f'Faction {faction["ID"]}: renown threshold is not universal data index 0')
    return faction['ReputationMax_0']


def unlock_description(conditions, condition_id):
    """Authored failure text; an encrypted or missing condition stays absent."""
    row = conditions.get(condition_id)
    return row['Failure_description_lang'] if row else None


def build_catalog(covenants, factions, currencies, rewards, conditions):
    by_covenant = {}
    for row in rewards:
        by_covenant.setdefault(row['CovenantID'], []).append(row)
    catalog = []
    for covenant in sorted(covenants, key=lambda row: row['ID']):
        faction = factions.get(covenant['FactionID'])
        if faction is None or faction['RenownCurrencyID'] <= 0:
            continue
        currency = currencies[faction['RenownCurrencyID']]
        ordered = sorted(by_covenant.get(covenant['ID'], []), key=lambda r: (r['Level'], r['UiOrder'], r['ID']))
        catalog.append({
            'id': covenant['ID'], 'faction_id': faction['ID'], 'name': faction['Name_lang'],
            'description': faction['Description_lang'], 'expansion': faction['Expansion'],
            'currency_id': currency['ID'], 'max_level': currency['MaxQty'],
            'level_threshold': level_threshold(faction),
            'player_condition_id': covenant['RequiredPlayerConditionID'],
            'unlock_description': unlock_description(conditions, covenant['RequiredPlayerConditionID']),
            'rewards': [{key: row[field] for field, key in REWARD_FIELDS.items()} for row in ordered],
        })
    return catalog


def read_source(directory, definitions, name):
    fdid, layout = SOURCES[name]
    TABLES[name] = (fdid, layout)
    path = directory / f'{fdid}.db2'
    columns, rows, dropped = read_table(name, path, definitions / f'{name}.dbd')
    names = [column for column, _ in columns]
    layout = struct.unpack_from('<I', path.read_bytes(), 156)[0]
    receipt = {'table': name, 'fdid': fdid, 'layout': f'{layout:08X}', 'records': len(rows),
               'unreadable': dropped, 'md5': hashlib.md5(path.read_bytes()).hexdigest()}
    return [dict(zip(names, row)) for row in rows], receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db2', type=Path, required=True, help='directory of extracted <fdid>.dat files')
    parser.add_argument('--definitions', type=Path, required=True, help='pinned WoWDBDefs definitions')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    tables, receipts = {}, []
    for name in SOURCES:
        tables[name], receipt = read_source(args.db2, args.definitions, name)
        receipts.append(receipt)
    catalog = build_catalog(tables['Covenant'], {row['ID']: row for row in tables['Faction']},
                            {row['ID']: row for row in tables['CurrencyTypes']}, tables['RenownRewards'],
                            {row['ID']: row for row in tables['PlayerCondition']})
    args.output.write_text(json.dumps({'build': source_identity(args.db2)['build'], 'sources': receipts, 'major_factions': catalog},
                                      indent=1, ensure_ascii=False) + '\n')
    print(json.dumps({'major_factions': len(catalog), 'rewards': sum(len(row['rewards']) for row in catalog)}))


if __name__ == '__main__':
    main()
