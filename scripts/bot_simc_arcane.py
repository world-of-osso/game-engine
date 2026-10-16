"""Pinned SimC projection for the captured level-32 Arcane kit.

Explicit per-spec priority provider for PlayerController.cast_next. Not a
general APL interpreter: other kits, talents, consumables, external buffs and
on-use items fail explicitly. Third-party-guide priorities are never used, and
there is no registry or generic fallback.
"""
import copy
import csv
import hashlib
import re
from pathlib import Path

COMMIT = "515f4a3fe9d1f35e1dbaf253745b6105fba906fb"
APL_SHA256 = "b1d85887ec62431864cc2db45b95993868bc02a0c0852b5189997542f47cabd4"
RAW_URL = (f"https://raw.githubusercontent.com/simulationcraft/simc/{COMMIT}"
           "/ActionPriorityLists/default/mage_arcane.simc")
SPEC, LEVEL = 62, 32
SUPPORTED_ACTIONS = {"counterspell", "arcane_blast", "arcane_barrage"}
COUNTERSPELL, ARCANE_BLAST, ARCANE_BARRAGE = 2139, 30451, 44425
ARCANE_CHARGES = "16"
FIGHT_STYLE, ITERATION_S = "DUNGEON_ROUTE", 2700


def native_bool(snapshot, field, spell):
    result = snapshot["facts"][field][str(spell)]
    if set(result) != {"Ok"} or not isinstance(result["Ok"], bool):
        raise ValueError(f"Native {field} for {spell} unavailable: {result}")
    return result["Ok"]


def usable(snapshot, spell):
    return all(native_bool(snapshot, field, spell) for field in ("ready", "affordable", "in_range"))


def read_rows(path, key, wanted, value):
    with Path(path).open(newline="") as source:
        return {int(row[key]): row[value] for row in csv.DictReader(source) if int(row[key]) in wanted}


def verify_source(receipt, apl_path):
    if (receipt.get("commit") != COMMIT or receipt.get("sha256") != APL_SHA256
            or receipt.get("raw_url") != RAW_URL):
        raise ValueError("Priority source receipt does not match approved pinned APL")
    apl = Path(apl_path).read_bytes()
    if hashlib.sha256(apl).hexdigest() != APL_SHA256:
        raise ValueError("Pinned APL bytes differ from source receipt")
    return apl.decode()


def trait_spells(config, db2_dir):
    """Resolve received server TraitConfigSnapshot entries to native DB2 spell IDs."""
    if not isinstance(config, dict) or config.get("spec_id") != SPEC or config.get("tree_id", 0) <= 0:
        raise ValueError("Authoritative server trait configuration required")
    selected = {int(entry["entry_id"]) for entry in config["entries"] if entry["rank"] > 0}
    definitions = read_rows(db2_dir / "TraitNodeEntry.csv", "ID", selected, "TraitDefinitionID")
    if definitions.keys() != selected:
        raise ValueError("Authoritative trait entry absent from native DB2")
    wanted = {int(definition) for definition in definitions.values()}
    spells = read_rows(db2_dir / "TraitDefinition.csv", "ID", wanted, "SpellID")
    if spells.keys() != wanted:
        raise ValueError("Authoritative trait definition absent from native DB2")
    return {int(spell) for spell in spells.values()} - {0}


def require_context(context):
    if context.get("potion") is not False or context.get("external_buffs") != [] \
            or context.get("on_use_items") != []:
        raise ValueError("Consumable/external/item APL coverage is not implemented")
    started = context.get("started_at_ms")
    if (context.get("fight_style") != FIGHT_STYLE or context.get("expected_iteration_s") != ITERATION_S
            or not isinstance(started, int) or isinstance(started, bool)):
        raise ValueError("Explicit DungeonRoute fight context is required")
    return started


class ArcaneSimcProvider:
    """Callable (snapshot, target) -> action|None over a frozen preflight."""

    def __init__(self, receipt, apl_path, captured_kit, db2_dir, initial_snapshot, context):
        text = verify_source(receipt, apl_path)
        if captured_kit.get("spec") != SPEC or captured_kit.get("level") != LEVEL:
            raise ValueError("Captured kit is not level-32 Arcane")
        self.known = frozenset(int(spell) for spell in captured_kit["known"])
        initial_spells = initial_snapshot["spells"]
        if (initial_spells.get("level") != LEVEL or initial_spells.get("spec") != SPEC
                or set(initial_spells["known"]) != self.known):
            raise ValueError("Initial native kit differs from the captured level-32 Arcane kit")
        if "trait_config" not in initial_snapshot:
            raise ValueError("Received server trait configuration snapshot required")
        self.trait_config = copy.deepcopy(initial_snapshot["trait_config"])
        self.started_at_ms = require_context(context)
        db2_dir = Path(db2_dir)
        traits = trait_spells(self.trait_config, db2_dir)
        # Literal pinned APL names, resolved from known IDs and server trait entries.
        actions = set(re.findall(r"^actions(?:\.\w+)?\+?=/?(\w+)", text, re.MULTILINE))
        talents = set(re.findall(r"talent\.([a-z_0-9]+)", text))
        wanted = set(self.known) | traits
        names = {spell: name.lower().replace(" ", "_")
                 for spell, name in read_rows(db2_dir / "SpellName.csv", "ID", wanted, "Name_lang").items()}
        if names.keys() != wanted:
            raise ValueError("Native kit spell names are incomplete")
        # Precombat buffs are excluded by the no-buffs scope.
        combat_names = {name for spell, name in names.items() if spell in self.known}
        combat_actions = actions & combat_names - {"arcane_intellect", "mirror_image"}
        selected_talents = sorted(spell for spell in traits if names[spell] in talents)
        if combat_actions != SUPPORTED_ACTIONS:
            raise ValueError(f"APL kit contains unsupported or missing actions: {sorted(combat_actions)}")
        if selected_talents:
            raise ValueError(f"Selected talent APL coverage is not implemented: {selected_talents}")
        self.context = {
            "fight_style": FIGHT_STYLE, "started_at_ms": self.started_at_ms,
            "expected_iteration_s": ITERATION_S, "potion": False, "external_buffs": [],
            "on_use_items": [], "known_apl_actions": sorted(combat_actions),
            "selected_talents": selected_talents, "trait_spells": sorted(traits),
            "native_spell_names_sha256": hashlib.sha256((db2_dir / "SpellName.csv").read_bytes()).hexdigest()}

    def fight_remains(self, snapshot, target):
        """Validate snapshot against the frozen preflight; return iteration seconds left."""
        if snapshot["target"] != int(target):
            raise ValueError("Native facts do not belong to the assisted target")
        spells = snapshot["spells"]
        if (spells.get("level") != LEVEL or spells["spec"] != SPEC
                or set(spells["known"]) != self.known):
            raise ValueError("Native kit differs from the captured level-32 Arcane kit")
        if set(snapshot["facts"]["known"]) != self.known:
            raise ValueError("Native learned-spell and evaluator kit snapshots disagree")
        if snapshot.get("trait_config") != self.trait_config:
            raise ValueError("Native authoritative trait configuration differs from preflight")
        elapsed = (snapshot["at_ms"] - self.started_at_ms) / 1000
        if elapsed < 0:
            raise ValueError("Declared fight context starts after the native snapshot")
        return ITERATION_S - elapsed

    def __call__(self, snapshot, target):
        remains = self.fight_remains(snapshot, target)
        if snapshot["spells"]["casting"] or snapshot["spells"]["gcd_ms"] > 0:
            return None
        cast = snapshot["target_cast"]
        # APL24, sc_mage.cpp3639-3660: interrupt actions require target casting.
        if (cast["casting"] or cast["channeling"]) and cast["interruptible"] and usable(snapshot, COUNTERSPELL):
            return action(COUNTERSPELL, 24)
        # APL36. fight_remains is iteration time, NOT this target's health/TTD.
        # sim.cpp3579-3583; DungeonRoute max_time is 45min at2360-2369.
        gcd = max(0.75, snapshot["spell_gcd_ms"][str(ARCANE_BARRAGE)] / 1000
                  / (1 + snapshot["player_haste_pct"] / 100))
        if remains < gcd * 2 and snapshot["facts"]["powers"][ARCANE_CHARGES] == 4 \
                and usable(snapshot, ARCANE_BARRAGE):
            return action(ARCANE_BARRAGE, 36)
        # Other damage actions in cooldowns/sunfury are unlearned in this exact
        # kit; spellfire_spheres/splintering_sorcery are unselected. APL64 filler.
        if usable(snapshot, ARCANE_BLAST):
            return action(ARCANE_BLAST, 64)
        return None


def action(spell, line):
    return {"spell": spell, "apl_line": line, "source_commit": COMMIT, "source_sha256": APL_SHA256}
