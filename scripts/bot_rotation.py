"""Generic highest-direct-damage DB2 coefficient rule, not a Retail priority list.

Deliberately limited to active, known, explicit enemy-target school damage.
No inferred priority for procs, periodic auras, pets, weapon attacks or utilities.
"""
import csv
from collections import defaultdict
from pathlib import Path


def authored_next(snapshot):
    """Use the existing sourced native evaluator; unknown predicates block, not fall back."""
    if not snapshot.get("available"):
        raise ValueError("Authored native rotation not loaded")
    if snapshot.get("failure_reason"):
        raise ValueError(snapshot["failure_reason"])
    return snapshot.get("next_spell") or None


def authored_order(directory, spec):
    identities = {int(row["ID"]) for row in read_rows(directory, "AssistedCombat")
                  if int(row["ChrSpecializationID"]) == spec}
    steps = [row for row in read_rows(directory, "AssistedCombatStep")
             if int(row["AssistedCombatID"]) in identities]
    return [int(row["SpellID"]) for row in sorted(steps, key=lambda row: int(row["OrderIndex"]))]


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


def load_effect_closure(directory, known):
    effects = {}
    pending = set(known)
    while pending:
        loaded = spell_rows(directory, "SpellEffect", pending)
        for identity in pending:
            effects[identity] = loaded[identity]
        triggered = {int(row["EffectTriggerSpell"]) for rows in loaded.values() for row in rows
                     if row["Effect"] in ("32", "64") and row["EffectTriggerSpell"] != "0"}
        pending = triggered - effects.keys()
    return effects


def school_damage_rows(effects, identity, visiting=frozenset()):
    if identity in visiting:
        raise ValueError(f"Cyclic direct damage trigger at spell {identity}")
    rows = effects.get(identity, [])
    result = [row for row in rows if row["Effect"] == "2" and row["ImplicitTarget_0"] == "6"]
    for row in rows:
        if row["Effect"] in ("32", "64") and row["EffectTriggerSpell"] != "0":
            result.extend(school_damage_rows(effects, int(row["EffectTriggerSpell"]), visiting | {identity}))
    return result


def load_spells(directory, known, direct_only=True):
    known = set(known)
    effects = load_effect_closure(directory, known)
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
        direct = school_damage_rows(effects, identity)
        coefficient = sum(float(row["EffectBonusCoefficient"]) + float(row["BonusCoefficientFromAP"]) for row in direct)
        if direct_only and coefficient <= 0:
            continue
        result.append({
            "id": identity, "damage": bool(direct),
            "damage_ids": sorted({int(row["SpellID"]) for row in direct}), "coefficient": coefficient,
            "cast_ms": max(times[int(row["CastingTimeIndex"])] for row in misc[identity]),
            "gcd_ms": max((int(row["StartRecoveryTime"]) for row in cooldowns[identity]), default=0),
            "cooldown_ms": max((max(int(row["RecoveryTime"]), int(row["CategoryRecoveryTime"])) for row in cooldowns[identity]), default=0),
            "source": {"SpellEffect": effects[identity], "damage_effects": direct, "SpellMisc": misc[identity],
                       "SpellCooldowns": cooldowns[identity], "SpellPower": power[identity]},
        })
    return result


def choose(spells, unavailable_until, now_ms):
    eligible = [spell for spell in spells if unavailable_until.get(spell["id"], 0) <= now_ms]
    return max(eligible, key=lambda spell: (spell["coefficient"], -spell["id"]), default=None)
