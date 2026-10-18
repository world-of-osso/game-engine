"""Source aggro radii and hideable hostile population of The Stockade (map 34, difficulty 1).

A read-only mirror of the game-server source inputs the pull corridor refusal needs:
`creature_aggro_distance.rs` (TC a352b1fa Creature::GetAttackDistance), shared-protocol
`level_scaling.rs` (level_for_viewer), `model_reach.rs` (CombatReach x UnitScale),
`world_data.rs` (template_for, spawns, addons), `zones.rs` (spawn level and display),
`faction_reaction.rs` and the ported `scripted_ai/the_stockade.rs` / `smart_ai.rs` casts.
Hiding is a MOD_STEALTH or MOD_INVISIBILITY aura (auras/stealth.rs, auras/invisibility.rs).
Anything whose source semantics are not modelled here raises ValueError.
"""
import csv
import sqlite3
import struct
from pathlib import Path

MAP_ID = 34
DIFFICULTY = 1
INSTANCE_SCRIPT = "instance_the_stockade"

# creature_aggro.rs / creature_aggro_distance.rs.
BASE_ATTACK_DISTANCE = 15.0
MIN_ATTACK_DISTANCE = 5.0
MAX_AGGRO_RADIUS = 45.0
PLAYER_COMBAT_REACH = 1.5
MAX_PLAYER_LEVEL = 90
# shared-protocol level_scaling.rs MAX_LEVEL; level_for_target clamps to MAX_LEVEL + 3.
MAX_SCALED_LEVEL = 123 + 3
# zones.rs DEFAULT_INVISIBLE_DISPLAY: a template without models.
DEFAULT_INVISIBLE_DISPLAY = 11_686

# auras/handlers.rs aura types.
AURA_DUMMY = 4
MOD_STEALTH = 16
MOD_INVISIBILITY = 18
MOD_SCALE = 61
MOD_DETECT_RANGE = 91
MOD_DETECTED_RANGE = 152
HIDE_AURAS = {MOD_STEALTH, MOD_INVISIBILITY}
# SpellEffect.Effect values.
EFFECT_DUMMY = 3
EFFECT_APPLY_AURA = 6
EFFECT_SUMMON = 28
EFFECT_SCRIPT_EFFECT = 77
TARGET_UNIT_CASTER = 1

# creature_aggro.rs AggroMode::for_template.
CREATURE_TYPE_CRITTER = 8
CREATURE_TYPE_NON_COMBAT_PET = 12
CREATURE_FLAG_EXTRA_CIVILIAN = 0x2
CREATURE_FLAG_EXTRA_TRIGGER = 0x80
UNIT_FLAG_IMMUNE_TO_PC = 0x100
TRAINING_DUMMY_SCRIPT = "npc_training_dummy"
# faction_reaction.rs HOSTILE_BY_DEFAULT.
HOSTILE_BY_DEFAULT = 0x2000

# scripted_ai/the_stockade.rs, the only map 34 ScriptNames the server ports (CREATURE_SCRIPTS):
# script -> (spells its CreatureAi casts, creatures it summons). No other aura source exists in
# them; ctx.remove_all_auras only removes.
STOCKADE_SCRIPTS = {
    # execute_event 124-138 (86604, 86620), damage_taken 180-186 (86736), just_died 156-163.
    "boss_hogger": ((86604, 86620, 86736), (46409,)),
    # NpcWardenThelwater: talk only.
    "npc_warden_thelwater": ((), ()),
    # execute_event 257-276.
    "boss_lord_overheat": ((12466, 86633, 86636), ()),
    # execute_event 359-389 (55964 Vanish, 55966 Shadowstep), just_died 423-427.
    "boss_randolph_moloch": ((86726, 86729, 55964, 55966), (46482,)),
    # NpcMortimerMoloch: talk and kill_self only.
    "npc_mortimer_moloch": ((), ()),
}

# scripted_ai/smart_ai.rs SMART_ACTION dispatch. Cast (11) applies its spell and a timed
# action list (80) runs source_type 9 rows; these apply no spell. 25 (FLEE_FOR_ASSIST) is
# outside supports_action, so the dispatch's `_ => {}` ignores it. Summons (12, 107),
# SET_FACTION (2) and every other action are unsupported here.
SMART_ACTION_CAST = 11
SMART_ACTION_CALL_TIMED_ACTION_LIST = 80
SMART_NO_SPELL_ACTIONS = {1, 6, 17, 22, 25, 29, 33, 34, 37, 41, 49, 53, 54, 55, 59, 63, 64, 65,
                          66, 81, 82, 83, 145, 152}
SMART_SOURCE_CREATURE = 0
SMART_SOURCE_TIMED_ACTION_LIST = 9


def f32(value):
    return struct.unpack("f", struct.pack("f", value))[0]


def expansion_max_level(expansion):
    """creature_aggro_distance.rs expansion_max_level (SharedDefines.h:109-140)."""
    for expansions, level in (((0, 1, 2), 30), ((3, 4), 35), ((5,), 40), ((6,), 45), ((7,), 50),
                              ((8,), 60), ((9,), 70), ((10,), 80), ((-1, 11), 90)):
        if expansion in expansions:
            return level
    raise ValueError(f"HealthScalingExpansion {expansion} has no source expansion level")


def level_for_target(min_level, max_level, delta, target_level):
    """level_scaling.rs LevelScaling::level_for_target."""
    clamped = min(max(target_level, min_level), max_level)
    return min(max(clamped + delta, 1), MAX_SCALED_LEVEL)


def center_radius(creature_level, target_level, creature_reach, target_reach, expansion_level,
                  range_modifiers=0.0):
    """creature_aggro_distance.rs AttackDistance::center_radius, in f32."""
    base = f32(BASE_ATTACK_DISTANCE - creature_reach)
    if creature_level > expansion_level:
        radius = f32(base + expansion_level - target_level)
    else:
        applies = creature_level + 5 <= MAX_PLAYER_LEVEL
        radius = f32(base + (creature_level - target_level) + (range_modifiers if applies else 0.0))
    radius = min(max(radius, MIN_ATTACK_DISTANCE), MAX_AGGRO_RADIUS)
    return f32(f32(radius + creature_reach) + target_reach)


def range_modifiers_apply(creature_level, expansion_level):
    return creature_level <= expansion_level and creature_level + 5 <= MAX_PLAYER_LEVEL


def _int(value):
    return int(float(value))


class PullSources:
    def __init__(self, world_db: Path, db2_dir: Path):
        self.world_db = Path(world_db)
        self.db2_dir = Path(db2_dir)
        self.db = sqlite3.connect(f"{self.world_db.resolve().as_uri()}?mode=ro", uri=True)
        self.db.row_factory = sqlite3.Row
        self._spells = self._load_spell_names()
        self._effects = self._load_effects()
        self._factions = self._load_factions()
        self._reaches = {row[0]: float(row[1]) for row in self.db.execute(
            "SELECT DisplayID, CombatReach FROM encounter_creature_model_info")}

    # --- source data -------------------------------------------------------------------
    def _load_spell_names(self):
        """spell_info/load.rs builds the server SpellStore from spell_name rows."""
        with open(self.db2_dir / "SpellName.csv", newline="") as handle:
            spells = {int(row["ID"]) for row in csv.DictReader(handle)}
        spells.update(row[0] for row in self.db.execute("SELECT ID FROM spell_name"))
        spells.update(row[0] for row in self.db.execute("SELECT Id FROM tdb_serverside_spell"))
        return spells

    def _load_effects(self):
        """(effect, aura, trigger, implicit target 0) of every difficulty's effect row."""
        effects = {}
        with open(self.db2_dir / "SpellEffect.csv", newline="") as handle:
            reader = csv.reader(handle)
            header = next(reader)
            columns = [header.index(name) for name in
                       ("SpellID", "Effect", "EffectAura", "EffectTriggerSpell", "ImplicitTarget_0")]
            for row in reader:
                spell, *values = (_int(row[index]) for index in columns)
                effects.setdefault(spell, []).append(tuple(values))
        for row in self.db.execute("SELECT SpellID, Effect, EffectAura, EffectTriggerSpell, "
                                   "ImplicitTarget1 FROM tdb_serverside_spell_effect"):
            effects.setdefault(row[0], []).append(tuple(row[1:]))
        return effects

    def _load_factions(self):
        with open(self.db2_dir / "FactionTemplate.csv", newline="") as handle:
            factions = {}
            for row in csv.DictReader(handle):
                factions[int(row["ID"])] = {
                    "faction": int(row["Faction"]), "flags": int(row["Flags"]),
                    "group": int(row["FactionGroup"]), "friend_group": int(row["FriendGroup"]),
                    "enemy_group": int(row["EnemyGroup"]),
                    "enemies": {int(row[f"Enemies_{i}"]) for i in range(8)},
                    "friends": {int(row[f"Friend_{i}"]) for i in range(8)},
                }
        return factions

    def _template(self, entry):
        """world_data.rs template_for(entry, difficulty): difficulty row over the base template."""
        base = self.db.execute("SELECT * FROM content_creature_template WHERE entry = ?",
                               (entry,)).fetchone()
        if base is None:
            raise ValueError(f"creature {entry} has no source template")
        template = dict(base)
        difficulty = self.db.execute(
            "SELECT minlevel, maxlevel, ContentTuningID, HealthScalingExpansion, LevelScalingDeltaMin,"
            " LevelScalingDeltaMax FROM content_creature_template_difficulty"
            " WHERE entry = ? AND DifficultyID = ?", (entry, DIFFICULTY)).fetchone()
        if difficulty is not None:
            template.update(dict(difficulty))
        return template

    def _reach(self, display):
        if display not in self._reaches:
            raise ValueError(f"display {display} has no source model CombatReach")
        return self._reaches[display]

    def _reaction(self, own, other):
        """faction_reaction.rs reaction; an unknown template is unsupported here."""
        for template in (own, other):
            if template not in self._factions:
                raise ValueError(f"FactionTemplate {template} is absent from active DB2")
        if own == other:
            return "friendly"
        a, b = self._factions[own], self._factions[other]
        if self._hostile(a, b):
            return "hostile"
        if self._friendly(a, b) or self._friendly(b, a):
            return "friendly"
        return "hostile" if a["flags"] & HOSTILE_BY_DEFAULT else "neutral"

    @staticmethod
    def _hostile(a, b):
        if b["faction"]:
            if b["faction"] in a["enemies"]:
                return True
            if b["faction"] in a["friends"]:
                return False
        return bool(a["enemy_group"] & b["group"])

    @staticmethod
    def _friendly(a, b):
        if a["faction"] == b["faction"]:
            return True
        if b["faction"]:
            if b["faction"] in a["enemies"]:
                return False
            if b["faction"] in a["friends"]:
                return True
        return bool(a["friend_group"] & b["group"] or a["group"] & b["friend_group"])

    # --- spells ------------------------------------------------------------------------
    def _aura_effects(self, auras):
        for aura in auras:
            spell = int(aura["spell_id"])
            if spell not in self._spells:
                raise ValueError(f"replicated aura {spell} is absent from the source spell store")
            yield spell, self._effects.get(spell, ())

    def _range_modifier_spells(self, auras, aura_type):
        return [spell for spell, effects in self._aura_effects(auras)
                if any(effect[1] == aura_type for effect in effects)]

    def _reachable(self, roots):
        """Spells reachable from `roots` through EffectTriggerSpell, and absent ids."""
        reached, absent, pending = set(), set(), list(roots)
        while pending:
            spell = pending.pop()
            if spell in reached or spell in absent:
                continue
            if spell not in self._spells:
                absent.add(spell)
                continue
            reached.add(spell)
            pending.extend(effect[2] for effect in self._effects.get(spell, ()) if effect[2])
        return reached, absent

    def _hides_caster(self, spell):
        hides = False
        for effect, aura, _trigger, target in self._effects.get(spell, ()):
            if effect in (EFFECT_DUMMY, EFFECT_SCRIPT_EFFECT) or aura == AURA_DUMMY:
                raise ValueError(f"spell {spell} has a script-defined effect {effect}/aura {aura}")
            if effect == EFFECT_SUMMON:
                raise ValueError(f"spell {spell} summons creatures outside the spawn population")
            if aura in (MOD_DETECT_RANGE, MOD_SCALE):
                raise ValueError(f"spell {spell} aura {aura} changes the source aggro radius")
            if aura in HIDE_AURAS:
                if effect != EFFECT_APPLY_AURA or target != TARGET_UNIT_CASTER:
                    raise ValueError(f"spell {spell} hides units other than its caster")
                hides = True
        return hides

    # --- creature AI casts -------------------------------------------------------------
    def _ai_sources(self, template, spawn_scripts):
        """(spells the creature's AI or data can apply, entries it summons)."""
        script, ai = template["ScriptName"] or "", template["AIName"] or ""
        if any(spawn_scripts):
            raise ValueError(f"creature {template['entry']} has a spawn ScriptName override")
        if script:
            if script not in STOCKADE_SCRIPTS:
                raise ValueError(f"creature {template['entry']} ScriptName {script!r} is unsupported")
            instance = self.db.execute("SELECT script FROM tdb_instance_template WHERE map = ?",
                                       (MAP_ID,)).fetchone()
            if instance is None or instance[0] != INSTANCE_SCRIPT:
                raise ValueError(f"map {MAP_ID} instance script is not {INSTANCE_SCRIPT}")
            spells, summons = STOCKADE_SCRIPTS[script]
            return set(spells), set(summons)
        if ai == "SmartAI":
            return self._smart_spells(SMART_SOURCE_CREATURE, template["entry"], set()), set()
        if ai:
            raise ValueError(f"creature {template['entry']} AIName {ai!r} is unsupported")
        return set(), set()

    def _smart_spells(self, source_type, key, visited):
        if (source_type, key) in visited:
            return set()
        visited.add((source_type, key))
        spells = set()
        for action, param in self.db.execute(
                "SELECT action_type, action_param1 FROM tdb_smart_scripts"
                " WHERE source_type = ? AND entryorguid = ?", (source_type, key)):
            if action == SMART_ACTION_CAST:
                spells.add(param)
            elif action == SMART_ACTION_CALL_TIMED_ACTION_LIST:
                spells |= self._smart_spells(SMART_SOURCE_TIMED_ACTION_LIST, param, visited)
            elif action not in SMART_NO_SPELL_ACTIONS:
                raise ValueError(f"SmartAI {key} action {action} is unsupported")
        return spells

    # --- population --------------------------------------------------------------------
    def _spawns(self):
        """content_creature rows of map 34 that spawn at difficulty 1."""
        rows = self.db.execute("SELECT guid, id1, spawnDifficulties, modelid, ScriptName"
                               " FROM content_creature WHERE map = ?", (MAP_ID,))
        return [row for row in rows if DIFFICULTY in
                {int(token) for token in str(row["spawnDifficulties"]).split(",")}]

    def _addon_auras(self, entry, guids):
        """world_data.rs addon_for: the spawn's addon row, else its template's."""
        template = self.db.execute("SELECT auras FROM content_creature_template_addon WHERE entry = ?",
                                   (entry,)).fetchone()
        rows = [self.db.execute("SELECT auras FROM content_creature_addon WHERE guid = ?",
                                (guid,)).fetchone() or template for guid in guids] or [template]
        return {int(token) for row in rows if row for token in (row[0] or "").split()}

    def _displays(self, entry, overrides):
        """zones.rs choose_display: a spawn modelid at scale 1, else any template model."""
        displays = {(override, 1.0) for override in overrides if override}
        if not all(overrides) or not overrides:
            models = {(row[0], float(row[1])) for row in self.db.execute(
                "SELECT CreatureDisplayID, DisplayScale FROM content_creature_template_model"
                " WHERE CreatureID = ?", (entry,))}
            displays |= models or {(DEFAULT_INVISIBLE_DISPLAY, 1.0)}
        return sorted(displays)

    def _levels(self, template, tank_level):
        """zones.rs spawn level: min level untuned, else level_for_target over the delta roll."""
        if not template["ContentTuningID"]:
            return [template["minlevel"]]
        low, high = sorted((template["LevelScalingDeltaMin"], template["LevelScalingDeltaMax"]))
        return sorted({level_for_target(template["minlevel"], template["maxlevel"], delta, tank_level)
                       for delta in range(low, high + 1)})

    @staticmethod
    def _aggressive(template):
        passive = (template["flags_extra"] & CREATURE_FLAG_EXTRA_TRIGGER
                   or template["type"] in (CREATURE_TYPE_CRITTER, CREATURE_TYPE_NON_COMBAT_PET)
                   or template["unit_flags"] & UNIT_FLAG_IMMUNE_TO_PC
                   or template["ScriptName"] == TRAINING_DUMMY_SCRIPT)
        return not passive and not template["flags_extra"] & CREATURE_FLAG_EXTRA_CIVILIAN

    def _candidates(self):
        """entry -> (spawn guids, display overrides, spawn ScriptNames), plus script summons."""
        candidates = {}
        for spawn in self._spawns():
            guids, overrides, scripts = candidates.setdefault(spawn["id1"], ([], [], []))
            guids.append(spawn["guid"])
            overrides.append(spawn["modelid"])
            scripts.append(spawn["ScriptName"])
        pending = list(candidates)
        while pending:
            template = self._template(pending.pop())
            if template["ScriptName"] in STOCKADE_SCRIPTS:
                for summon in STOCKADE_SCRIPTS[template["ScriptName"]][1]:
                    if summon not in candidates:
                        candidates[summon] = ([], [], [])
                        pending.append(summon)
        return candidates

    def _tank(self, tank):
        missing = [key for key in ("level", "scale", "faction", "auras") if tank.get(key) is None]
        if missing:
            raise ValueError(f"tank is missing replicated {missing}")
        return int(tank["level"]), f32(PLAYER_COMBAT_REACH * float(tank["scale"]))

    def population(self, tank: dict) -> dict:
        tank_level, tank_reach = self._tank(tank)
        if self._range_modifier_spells(tank["auras"], MOD_DETECTED_RANGE):
            raise ValueError("tank MOD_DETECTED_RANGE amount depends on caster spell modifiers")
        sources, absent = {}, set()
        for entry, (guids, overrides, scripts) in self._candidates().items():
            template = self._template(entry)
            if not self._aggressive(template):
                continue
            if self._reaction(template["faction"], int(tank["faction"])) != "hostile":
                continue
            roots, _summons = self._ai_sources(template, scripts)
            roots |= self._addon_auras(entry, guids)
            roots |= {row[0] for row in self.db.execute(
                "SELECT Spell FROM tdb_creature_template_spell WHERE CreatureID = ?", (entry,))}
            reached, missing = self._reachable(roots)
            absent |= missing
            hide_spells = sorted(spell for spell in reached if self._hides_caster(spell))
            expansion = expansion_max_level(template["HealthScalingExpansion"])
            displays = self._displays(entry, overrides)
            levels = self._levels(template, tank_level)
            radius = max(center_radius(level, tank_level, f32(self._reach(display) * scale),
                                       tank_reach, expansion)
                         for display, scale in displays for level in levels)
            sources[entry] = {"name": template["name"], "script": template["ScriptName"] or "",
                              "ai": template["AIName"] or "", "spells": sorted(reached),
                              "hide_spells": hide_spells, "displays": [list(d) for d in displays],
                              "levels": [levels[0], levels[-1]], "max_radius": radius}
        return {"map": MAP_ID, "difficulty": DIFFICULTY, "entries": sorted(sources),
                "hideable_entries": sorted(e for e, s in sources.items() if s["hide_spells"]),
                "max_radius": max((s["max_radius"] for s in sources.values()), default=0.0),
                "absent_spells": sorted(absent), "sources": sources}

    # --- one live creature -------------------------------------------------------------
    def radius(self, creature: dict, tank: dict) -> float:
        """Center distance below which `creature` aggroes on `tank` (aggro_on_sight_system)."""
        tank_level, tank_reach = self._tank(tank)
        template = self._template(int(creature["entry"]))
        expansion = expansion_max_level(template["HealthScalingExpansion"])
        scaling = creature.get("scaling")
        level = int(creature["level"]) if scaling is None else level_for_target(
            int(scaling["min_level"]), int(scaling["max_level"]), int(scaling["delta"]), tank_level)
        reach = f32(self._reach(int(creature["display"])) * float(creature["scale"]))
        detect = self._range_modifier_spells(creature.get("auras") or [], MOD_DETECT_RANGE)
        detected = self._range_modifier_spells(tank["auras"], MOD_DETECTED_RANGE)
        if (detect or detected) and range_modifiers_apply(level, expansion):
            raise ValueError(f"detection range auras {detect + detected} depend on caster spell modifiers")
        return center_radius(level, tank_level, reach, tank_reach, expansion)
