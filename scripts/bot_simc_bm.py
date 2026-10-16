"""Explicit pinned SimC projection for the captured level-32 BM single-target kit.

No implicit selection, reordered Kill Command predicate or alternate rotation.
Auto Shot is started through native75; native repetition is a separate live gate.
"""
import copy
import csv
import hashlib
import math
import re
from collections import Counter
from pathlib import Path

from bot_controller import native_usable

COMMIT = "515f4a3fe9d1f35e1dbaf253745b6105fba906fb"
SHA256 = "e38e17183f3638cb1d2680df5609367543f8fb91c1f59214799294ab8ced9442"
RAW_URL = (f"https://raw.githubusercontent.com/simulationcraft/simc/{COMMIT}"
           "/ActionPriorityLists/default/hunter_beast_mastery.simc")
AUTO_SHOT, WRATH, KILL_COMMAND, COBRA_SHOT, BARBED_SHOT = 75, 19574, 34026, 193455, 217200
ACTIONS = {"auto_shot", "bestial_wrath", "kill_command", "cobra_shot", "barbed_shot"}
HOWL_READY = {471878, 472324, 472325}
HOWL_COOLDOWN, COBRA_FANG = 471877, 1299389


def normalized(name):
    return re.sub(r"[^a-z0-9_]+", "", name.lower().replace(" ", "_"))


def read_rows(directory, table):
    with (Path(directory) / f"{table}.csv").open(newline="") as source:
        return {int(row["ID"]): row for row in csv.DictReader(source)}


def load_trait_names(config, directory, names):
    if config.get("spec_id") != 253 or config.get("tree_id", 0) <= 0:
        raise ValueError("Authoritative BM trait configuration required")
    entries = read_rows(directory, "TraitNodeEntry")
    definitions = read_rows(directory, "TraitDefinition")
    try:
        spells = {int(definitions[int(entries[int(row["entry_id"])]["TraitDefinitionID"])]["SpellID"])
                  for row in config["entries"] if row["rank"] > 0}
        return {normalized(names[spell]["Name_lang"]) for spell in spells if spell}
    except KeyError as error:
        raise ValueError(f"Native BM trait metadata absent: {error}") from error


def charge_clock(state):
    """SimC full_recharge_time and charges_fractional, in native milliseconds."""
    try:
        current, maximum, recharge, remaining = (float(state[key]) for key in
            ("current", "max", "recharge_ms", "remaining_ms"))
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError("Native timed charge authority required") from error
    if (not all(math.isfinite(value) for value in (current, maximum, recharge, remaining))
            or not 0 <= current <= maximum or maximum < 1 or recharge <= 0 or remaining < 0):
        raise ValueError(f"Invalid native charge clock: {state}")
    if current == maximum:
        return 0, maximum
    if remaining > recharge:
        raise ValueError(f"Native charge recovery exceeds its period: {state}")
    return remaining + (maximum - current - 1) * recharge, current + 1 - remaining / recharge


def auto_shot_successes(snapshot):
    return Counter(int(row["at_ms"]) for row in snapshot["spells"]["cast_success_events"]
                   if int(row["source"]) == int(snapshot["actor"])
                   and int(row["spell_id"]) == AUTO_SHOT)


class BeastMasterySimcProvider:
    def __init__(self, receipt, apl_path, captured_kit, db2_dir, initial_snapshot, context):
        if (receipt.get("commit") != COMMIT or receipt.get("sha256") != SHA256
                or receipt.get("raw_url") != RAW_URL):
            raise ValueError("BM source receipt differs from pinned upstream APL")
        apl = Path(apl_path).read_bytes()
        if hashlib.sha256(apl).hexdigest() != SHA256:
            raise ValueError("BM source bytes differ from pinned upstream APL")
        if (context.get("active_enemies") != 1 or context.get("potion") is not False
                or context.get("external_buffs") != [] or context.get("on_use_items") != []):
            raise ValueError("BM provider requires explicit single-target/no-item/no-external context")
        if captured_kit.get("spec") != 253 or captured_kit.get("level") != 32:
            raise ValueError("BM provider covers the captured level-32 kit only")
        self.known = frozenset(map(int, captured_kit["known"]))
        self.traits = copy.deepcopy(initial_snapshot["trait_config"])
        self.actor = int(initial_snapshot["actor"])
        names = read_rows(db2_dir, "SpellName")
        self.verify_kit(apl.decode(), db2_dir, names)
        self.auto_shot_before = auto_shot_successes(initial_snapshot)
        self.st_lines = [line for line, text in enumerate(apl.decode().splitlines(), 1)
                         if re.match(r"actions\.st\+?=", text)]
        self.auto_line = next(line for line, text in enumerate(apl.decode().splitlines(), 1)
                              if text.startswith("actions+=/auto_shot"))
        self.context = {**context, "commit": COMMIT, "sha256": SHA256, "raw_url": RAW_URL,
                        "known_apl_actions": sorted(ACTIONS), "selected_apl_talents": [],
                        "native_auto_shot_repetition": "requires live verification"}
        self.validate(initial_snapshot, initial_snapshot["target"])

    def verify_kit(self, text, directory, names):
        try:
            known_names = {normalized(names[spell]["Name_lang"]) for spell in self.known}
        except KeyError as error:
            raise ValueError(f"Native BM spell names absent: {error}") from error
        apl_actions = set(re.findall(r"^actions(?:\.\w+)?\+?=/?(\w+)", text, re.MULTILINE))
        if apl_actions & known_names != ACTIONS:
            raise ValueError(f"Unsupported BM APL actions: {sorted(apl_actions & known_names)}")
        selected = load_trait_names(self.traits, directory, names)
        referenced = set(re.findall(r"talent\.([a-z_0-9]+)", text))
        unsupported = selected & (referenced | {"natures_ally", "howl_of_the_pack_leader"})
        if unsupported:
            raise ValueError(f"BM selected talent predicates not covered: {sorted(unsupported)}")

    def validate(self, state, target):
        spells = state["spells"]
        if state["target"] != int(target) or int(state["actor"]) != self.actor:
            raise ValueError("BM native facts belong to another actor/target")
        if state.get("trait_config") != self.traits:
            raise ValueError("BM authoritative trait configuration changed")
        if (spells.get("level") != 32 or spells.get("spec") != 253
                or set(map(int, spells["known"])) != self.known
                or set(map(int, state["facts"]["known"])) != self.known):
            raise ValueError("BM native kit differs from captured preflight")
        if not state["facts"]["has_pet"]:
            raise ValueError("BM rotation requires native owned pet")
        auras = set(map(int, state["facts"]["player_auras"]))
        if HOWL_COOLDOWN in auras or COBRA_FANG in auras:
            raise ValueError("BM active Howl/Cobra Fang requires native duration/stack observability")

    @staticmethod
    def clock(state, spell):
        try:
            return charge_clock(state["spell_charge_state"][str(spell)])
        except KeyError as error:
            raise ValueError(f"Native timed charge state absent for {spell}") from error

    @staticmethod
    def usable(state, spell):
        if not native_usable(state, spell):
            return False
        if spell in (BARBED_SHOT, KILL_COMMAND):
            BeastMasterySimcProvider.clock(state, spell)
            return state["spell_charge_state"][str(spell)]["current"] > 0
        return True

    @staticmethod
    def action(spell, line):
        return {"spell": spell, "apl_line": line, "source_commit": COMMIT, "source_sha256": SHA256}

    def __call__(self, state, target):
        self.validate(state, target)
        if state["spells"]["casting"] or state["spells"]["gcd_ms"] > 0:
            return None
        if not auto_shot_successes(state) - self.auto_shot_before:
            return self.action(AUTO_SHOT, self.auto_line) if self.usable(state, AUTO_SHOT) else None
        full_barbed, _ = self.clock(state, BARBED_SHOT)
        gcd = max(750, state["spell_gcd_ms"][str(BARBED_SHOT)] / (1 + state["player_haste_pct"] / 100))
        wrath = state["facts"]["cooldowns_ms"][str(WRATH)]
        # actions.st, unchanged order and predicates; no selected Black Arrow,
        # Beast Cleave, Serpentine Strikes or Nature's Ally/apex.3 in this kit.
        if (wrath < gcd or full_barbed < gcd) and self.usable(state, BARBED_SHOT):
            return self.action(BARBED_SHOT, self.st_lines[0])
        if self.usable(state, WRATH):
            return self.action(WRATH, self.st_lines[1])
        # Wild Thrash has active_enemies>1; false in the explicit dummy context.
        _, fractional_kill = self.clock(state, KILL_COMMAND)
        howl_ready = bool(HOWL_READY & set(map(int, state["facts"]["player_auras"])))
        # !apex.3 is true; absent Howl cooldown has zero remains. Do not erase
        # the source's final conjunction merely because live KC has max1.
        if (howl_ready or fractional_kill > 1.8) and self.usable(state, KILL_COMMAND):
            return self.action(KILL_COMMAND, self.st_lines[3])
        # Cobra Fang at_max_stacks is false while that native aura is absent.
        if (state["facts"]["powers"]["2"] < 75 or full_barbed < gcd) and self.usable(state, BARBED_SHOT):
            return self.action(BARBED_SHOT, self.st_lines[5])
        if wrath > gcd and self.usable(state, COBRA_SHOT):
            return self.action(COBRA_SHOT, self.st_lines[6])
        return None
