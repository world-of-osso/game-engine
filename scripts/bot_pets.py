"""Explicit BM gameplay preparation from a real spawn and installed CreatureFamily."""
import csv
import math
import sqlite3
from contextlib import closing
from pathlib import Path

from bot_controller import require_point
from bot_stockade import parse_npc

TAME_BEAST = 1515  # Existing hunter-taming contract: channel1515 triggers13481/TAMECREATURE55.
PET_REACT_ASSIST, ACT_REACTION = 3, 6  # shared::protocol::pet_messages; received packed bar.


def owned_pet(state):
    pet = state['pet']
    native = pet.get('local_pet')
    announced = pet.get('pet')
    if native is not None and announced is not None and int(native) == int(announced):
        return str(int(native))
    return None


def assist_plan(state):
    pet = owned_pet(state)
    if pet is None:
        raise ValueError('Native Assist requires the replicated owned pet and its received bar')
    bar = state['pet']
    if bar['react_state'] == PET_REACT_ASSIST:
        return None
    slots = [index for index, packed in enumerate(bar['buttons'])
             if int(packed) >> 24 == ACT_REACTION and int(packed) & 0xffffff == PET_REACT_ASSIST]
    if len(slots) != 1:
        raise ValueError('Native received pet bar requires one Assist reaction slot')
    return {'pet': pet, 'name': f'PetActionButton{slots[0] + 1}'}


def verified_assist(receipt, pet):
    after = receipt['after']
    if owned_pet({'pet': after}) != str(pet) or after['react_state'] != PET_REACT_ASSIST:
        raise ValueError('Native owned pet Assist acknowledgement not observed')
    return after


def tame_plan(database, family_csv, map_id, level, position, known):
    require_point(position)
    if TAME_BEAST not in known:
        raise ValueError('Native learned Tame Beast1515 required for gameplay preparation')
    with Path(family_csv).open(encoding='utf-8-sig') as source:
        families = {int(row['ID']): row['Name_lang'] for row in csv.DictReader(source)}
    with closing(sqlite3.connect(Path(database).resolve().as_uri() + '?mode=ro', uri=True)) as db:
        rows = db.execute('SELECT c.guid,c.id1,t.name,t.family,c.position_x,c.position_y,c.position_z '
                          'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                          'WHERE c.map=? AND t.type=1 AND t.type_flags & 1 != 0 '
                          'AND t.type_flags & 65536 = 0 AND t.minlevel<=?', (int(map_id), int(level))).fetchall()
    if not rows:
        raise ValueError('No sourced ordinary tameable Beast spawn on the native map')
    guid, entry, name, family, x, y, z = min(rows, key=lambda row: math.dist(position, [row[4], row[6], -row[5]]))
    if family not in families:
        raise ValueError(f'Sourced creature family{family} absent from installed CreatureFamily')
    return {'spell': TAME_BEAST, 'entry': entry, 'name': name, 'spawn_guid': guid,
            'map': int(map_id), 'level': int(level), 'family': {'id': family, 'name': families[family]},
            'goal': [x, z, -y], 'selection': 'nearest authored non-exotic tameable Beast spawn; native actor verified before cast'}


def verified_tame_target(state, text, plan):
    target = state['target']
    if target.get('target_name') != plan['name'] or state['spells'].get('target_health', 0) <= 0:
        raise ValueError('Native tame target identity/alive health not observed')
    rows = [parse_npc(line, plan['entry']) for line in text.splitlines() if '(entity ' in line]
    selected = [row for row in rows if int(row['entity']) == int(target['target'])]
    if len(selected) != 1 or selected[0]['health'] <= 0 or selected[0]['level'] > plan['level']:
        raise ValueError('Unique alive native tame target at or below hunter level required')
    return selected[0]
