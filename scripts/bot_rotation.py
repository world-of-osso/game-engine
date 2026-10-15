"""Generic highest-direct-damage DB2 coefficient rule, not a Retail priority list.

Deliberately limited to active, known, explicit enemy-target school damage.
No inferred priority for procs, periodic auras, pets, weapon attacks or utilities.
"""
import csv
from collections import defaultdict
from pathlib import Path


def read_rows(directory, name):
    with (Path(directory) / f"{name}.csv").open() as source:
        yield from csv.DictReader(source)


def spell_rows(directory, name, known):
    result = defaultdict(list)
    for row in read_rows(directory, name):
        spell = int(row["SpellID"])
        if spell in known and row.get("DifficultyID", "0") == "0":
            result[spell].append(row)
    return result


def load_spells(directory, known):
    known = set(known)
    effects = spell_rows(directory, "SpellEffect", known)
    misc = spell_rows(directory, "SpellMisc", known)
    cooldowns = spell_rows(directory, "SpellCooldowns", known)
    power = spell_rows(directory, "SpellPower", known)
    times = {int(row["ID"]): int(row["Base"]) for row in read_rows(directory, "SpellCastTimes")}
    result = []
    for identity in sorted(known):
        if not misc[identity]:
            continue
        if any(int(row["Attributes_0"]) & 0x40 for row in misc[identity]):
            continue  # TC SPELL_ATTR0_PASSIVE; never cast a learned passive.
        direct = [row for row in effects[identity] if row["Effect"] == "2" and row["ImplicitTarget_0"] == "6"]
        if not direct:
            continue
        coefficient = sum(float(row["EffectBonusCoefficient"]) + float(row["BonusCoefficientFromAP"]) for row in direct)
        if coefficient <= 0:
            continue
        result.append({
            "id": identity, "damage": True, "damage_ids": [identity], "coefficient": coefficient,
            "cast_ms": max(times[int(row["CastingTimeIndex"])] for row in misc[identity]),
            "gcd_ms": max((int(row["StartRecoveryTime"]) for row in cooldowns[identity]), default=0),
            "cooldown_ms": max((max(int(row["RecoveryTime"]), int(row["CategoryRecoveryTime"])) for row in cooldowns[identity]), default=0),
            "source": {"SpellEffect": direct, "SpellMisc": misc[identity],
                       "SpellCooldowns": cooldowns[identity], "SpellPower": power[identity]},
        })
    return result


def choose(spells, unavailable_until, now_ms):
    eligible = [spell for spell in spells if unavailable_until.get(spell["id"], 0) <= now_ms]
    return max(eligible, key=lambda spell: (spell["coefficient"], -spell["id"]), default=None)
