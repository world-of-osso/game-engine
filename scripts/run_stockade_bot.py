"""Control one owned native Stockade party; caller owns processes and one admission.

No provisioning, stat changes, teleports or private diagnostic driver imports.
"""
import argparse
import json
import math
import os
import re
import sqlite3
import subprocess
import time
from collections import Counter
from contextlib import closing
from pathlib import Path

from bot_controller import publish
from bot_dungeon import summarize_dungeon
from bot_gear import equipment_receipt, load_plan
from bot_harness import parse_combat, poll_has_gap, routed_player
from bot_health import require_living_player
from bot_stockade import admitted_copy, authored_priority, encounter_states, parse_npc, parse_party, stockade_exit_plan, stockade_plan, target_observation_ready
from run_dummy_bots import observed_pet_owner


def position(account):
    values = account["local_player_position"].strip("()").split(",")
    point = [float(value) for value in values]
    if len(point) != 3 or not all(math.isfinite(value) for value in point):
        raise ValueError("Native player position unavailable")
    return point


class StockadeRun:
    def __init__(self, args):
        self.args = args
        self.output = args.output
        self.sequence = 0
        self.events = Counter()
        self.last_sequence = 0
        self.last_errors = []
        self.credit_offset = 0
        self.blocked_steps = 0
        self.plan = stockade_plan(args.world_db)
        self.run = {"spec": args.spec, "role": 2, "actor": None, "party_actors": [],
                    "start_ms": int(time.time() * 1000), "end_ms": None,
                    "elapsed_ms": 0, "clear_elapsed_ms": None, "capture_complete": True,
                    "bosses": self.plan["bosses"], "attempts": [], "native_errors": [],
                    "owner_snapshot": [], "capture_gaps": [], "failure": None}
        self.player = routed_player(self.invoke, self.snapshot,
                                   [str(args.engine / "scripts/agent/agent-run"), "bot-harness",
                                    str(args.server / "target/debug/examples/follower_route")],
                                   args.navmesh, self.output)
        publish(self.output / "source-plan.json", self.plan)

    def call(self, program, words, environment=None):
        self.sequence += 1
        result = subprocess.run([str(program), *map(str, words)], capture_output=True, text=True,
                                timeout=30, env=environment)
        receipt = {"at_ms": int(time.time() * 1000), "argv": [str(program), *map(str, words)],
                   "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
        publish(self.output / f"command-{self.sequence:06d}.json", receipt)
        if result.returncode:
            raise RuntimeError(f"Native command failed: {receipt}")
        return result.stdout

    def invoke(self, *words):
        if words[:2] == ("spell", "cast"):
            spell = int(words[words.index("--spell") + 1])
            target = int(words[words.index("--target") + 1])
            self.run["attempts"].append({"at_ms": int(time.time() * 1000), "spell": spell, "target": target})
        return self.call(self.args.engine / "target/debug/game-engine-cli",
                         ["--socket", self.args.socket, *words])

    def admin(self, *words):
        environment = dict(os.environ, GAME_SERVER_ADMIN_SOCKET="/tmp/game-server-admin-5871.sock")
        return self.call(self.args.server / "target/debug/game-server-admin", words, environment)

    def account(self):
        return json.loads((self.output / "native-account-state.json").read_text())

    def rotation(self):
        state = json.loads((self.output / "native-rotation-state.json").read_text())
        if int(time.time() * 1000) - state["at_ms"] > 10000:
            raise RuntimeError("Native controller observation is stale by more than10s")
        return state

    def snapshot(self, target):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            state = self.rotation()
            if target_observation_ready(state, target):
                if state["spells"]["spec"] != self.args.spec:
                    raise ValueError("Native specialization changed during the run")
                with (self.output / "priority-samples.jsonl").open("a") as stream:
                    stream.write(json.dumps(state) + "\n")
                return state
            time.sleep(.05)
        raise RuntimeError("Fresh target-bound native authored facts not observed within10s")

    def poll_combat(self):
        state = self.rotation()
        spells = state["spells"]
        text = self.invoke("combat", "log", "--lines", "200")
        self.events |= Counter(row["text"] for row in parse_combat(text))
        if poll_has_gap(self.last_sequence, spells["combat_log_seq"], spells["combat_log_count"]):
            self.run["capture_complete"] = False
            self.run["capture_gaps"].append({"at_ms": state["at_ms"], "previous_sequence": self.last_sequence,
                                             "sequence": spells["combat_log_seq"], "retained": spells["combat_log_count"]})
        self.last_sequence = spells["combat_log_seq"]
        errors = spells["errors"]
        if errors != self.last_errors:
            fresh = errors[len(self.last_errors):] if errors[:len(self.last_errors)] == self.last_errors else errors
            self.run["native_errors"].append({"at_ms": state["at_ms"], "errors": fresh})
            self.last_errors = list(errors)
        relation = observed_pet_owner({"pet": state["pet"], "account": self.account(), "at_ms": state["at_ms"]})
        if relation and not any(row["unit"] == relation["unit"] for row in self.run["owner_snapshot"]):
            self.run["owner_snapshot"].append(relation)
        with (self.output / "native-samples.jsonl").open("a") as stream:
            stream.write(json.dumps(state) + "\n")

    def owner_ready(self, account, state):
        named_owner = account.get("selected_character_name") == self.args.character
        entered = account.get("screen") == "InWorld" and account.get("world_map_id") == 34
        observed = account.get("local_player_health") is not None and state["spells"]["catalog_ready"]
        if not (named_owner and entered and observed):
            return False
        require_living_player(account)
        if state["spells"]["spec"] != self.args.spec:
            raise ValueError("Dungeon character is not the requested persisted specialization")
        return True

    def confirm_preflight(self, account, state):
        self.verify_equipment(state)
        self.bind_party(account)
        self.run["actor"] = str(account["local_player_id"])
        receipt = json.loads((self.output / "entry-confirmation.json").read_text())
        self.run["start_ms"] = receipt["at_ms"]
        self.entered_monotonic = receipt["monotonic"]
        self.last_errors = list(state["spells"]["errors"])
        self.poll_combat()

    def preflight(self):
        paths = [self.output / name for name in ["native-account-state.json", "native-rotation-state.json",
                                                "native-sheet-stats.json", "entry-confirmation.json"]]
        deadline = time.monotonic() + 240
        while time.monotonic() < deadline:
            if not all(path.exists() for path in paths):
                time.sleep(.2)
                continue
            account, state = self.account(), self.rotation()
            if not self.owner_ready(account, state):
                time.sleep(.2)
                continue
            roster = self.invoke("group", "roster")
            if roster.count(" hp=") != 5:
                publish(self.output / "preflight-pending.json", {"at_ms": state["at_ms"], "roster": roster})
                time.sleep(.2)
                continue
            self.confirm_preflight(account, state)
            return
        raise RuntimeError("Native Stockade entry/equipment preflight exceeded240s")

    def verify_equipment(self, state):
        plan = load_plan(self.args.world_db, self.args.engine / "data/db2/active", self.args.spec, state["spells"]["level"])
        sheet = json.loads((self.output / "native-sheet-stats.json").read_text())["sheet_stats"]
        receipt = equipment_receipt(plan, state["inventory"], sheet)
        publish(self.output / "fixture-receipt.json", receipt)
        publish(self.output / "equipment-plan.json", plan)
        stats = self.invoke("status", "character-stats")
        (self.output / "character-stats-before.txt").write_text(stats)

    def bind_party(self, account):
        party = parse_party(self.invoke("group", "roster"))
        if len(party) != 5 or Counter(row["role"] for row in party) != {"tank": 1, "healer": 1, "damage": 3}:
            raise ValueError("Native Stockade requires the real five-member1/1/3 group")
        self.tank = next(row["name"] for row in party if row["role"] == "tank")
        self.instance = admitted_copy(self.args.server_log.read_text(), account["selected_character_id"])
        actors = []
        with closing(sqlite3.connect(self.args.world_db.resolve().as_uri() + "?mode=ro", uri=True)) as db:
            for member in party:
                if member["name"] == self.args.character:
                    continue
                row = db.execute("SELECT entry FROM follower_profiles WHERE name=?", (member["name"],)).fetchone()
                if row is None:
                    raise ValueError("Native group member lacks its authored follower identity")
                actors.append(parse_npc(self.admin("npc-info", 34, self.instance, row[0]), row[0]))
        self.run["party_actors"] = [row["entity"] for row in actors]
        for boss in self.run["bosses"]:
            native = parse_npc(self.admin("npc-info", 34, self.instance, boss["entry"]), boss["entry"])
            if native["health"] <= 0 or native["difficulty"] != 1:
                raise ValueError("Expected fresh Normal boss was not alive")
            boss.update(entity=native["entity"], native_before=native)
        publish(self.output / "admitted-party.json", {"instance": self.instance, "party": party, "actors": actors})

    def record_credit(self):
        with self.args.server_log.open() as source:
            source.seek(self.credit_offset)
            text = source.read()
            self.credit_offset = source.tell()
        observed = encounter_states(text, self.instance)
        for boss in self.run["bosses"]:
            boss["state"] = observed.get(boss["encounter"], boss["state"])
        if all(boss["state"] == "Done" for boss in self.run["bosses"]) and self.run["clear_elapsed_ms"] is None:
            self.run["clear_elapsed_ms"] = int((time.monotonic() - self.entered_monotonic) * 1000)
        publish(self.output / "encounter-observations.json", {"at_ms": int(time.time() * 1000), "instance": self.instance,
                                                             "states": {row["encounter"]: row["state"] for row in self.run["bosses"]}})

    def party(self):
        account = self.account()
        require_living_player(account)
        if account.get("world_map_id") != 34:
            raise RuntimeError("Native player left the Stockade map")
        party = parse_party(self.invoke("group", "roster"))
        if len(party) != 5 or any(not row["online"] or row["health"] <= 0 or row["death"] != "Alive" for row in party):
            raise RuntimeError("Native party death/disconnect; no stat reset or replacement")
        return party

    def move(self, here, goal, label):
        if self.player.step_toward(here, goal, label):
            return
        time.sleep(.7)
        after = position(self.account())
        progressed = math.hypot(after[0] - here[0], after[2] - here[2]) >= .15
        self.blocked_steps = 0 if progressed else self.blocked_steps + 1
        # Preserve the maintained walk_route eight-step native failure boundary.
        if self.blocked_steps >= 8:
            raise RuntimeError(f"Native route blocked after eight steps at {after}; goal {goal}")

    def step(self):
        self.poll_combat()
        self.record_credit()
        if self.run["clear_elapsed_ms"] is not None:
            return True
        party = self.party()
        target = self.player.assist(self.tank)
        here = position(self.account())
        if target is not None:
            state = self.snapshot(target)
            if state["spells"]["target_health"] == 0:
                time.sleep(.2)
                return False
            goal = state["target_position"]
            if len(goal) != 3:
                raise RuntimeError("Native assisted target origin not observed")
            gap = math.hypot(goal[0] - here[0], goal[2] - here[2])
            if gap > 2.5:
                endpoint = [goal[0] + (here[0] - goal[0]) * 2.5 / gap, goal[1],
                            goal[2] + (here[2] - goal[2]) * 2.5 / gap]
                self.move(here, endpoint, "native-melee-approach")
            else:
                self.player.cast_next(target, authored_priority)
        elif all(row["health"] >= row["max_health"] for row in party):
            goal = next(boss["goal"] for boss in self.run["bosses"] if boss["state"] != "Done")
            self.move(here, goal, "authored-boss-goal")
        time.sleep(.2)
        return False

    def exit_instance(self):
        plan = stockade_exit_plan(self.args.world_db)
        publish(self.output / "source-exit-plan.json", plan)
        deadline = time.monotonic() + 240
        while time.monotonic() < deadline:
            path = self.output / "native-account-state.json"
            if not path.exists():
                time.sleep(.2)
                continue
            account = self.account()
            if account.get("screen") != "InWorld":
                time.sleep(.2)
                continue
            if account.get("selected_character_name") != self.args.character:
                raise ValueError("Exit preparation selected another character")
            require_living_player(account)
            if account.get("world_map_id") == plan["destination_map"]:
                publish(self.output / "exit-confirmation.json", {"at_ms": int(time.time() * 1000),
                        "account": account, "source": plan, "basis": "native walking through authored area trigger"})
                return
            if account.get("world_map_id") != 34:
                raise ValueError("Exit preparation encountered an unexpected map")
            here = position(account)
            # Trigger centers are not ground heights. At the nav endpoint, await
            # the ordinary map transition instead of requesting vertical movement.
            if math.hypot(here[0] - plan["goal"][0], here[2] - plan["goal"][2]) < .5:
                time.sleep(.2)
                continue
            self.move(here, plan["goal"], "authored-exit-trigger")
        raise TimeoutError("Ordinary Stockade exit exceeded240s")

    def execute(self):
        try:
            self.preflight()
            deadline = self.entered_monotonic + self.args.timeout
            while time.monotonic() < deadline:
                if self.step():
                    return
            raise TimeoutError("Stockade run exceeded the explicit wall-time bound")
        except Exception as error:
            self.run["failure"] = str(error)
            raise
        finally:
            self.finish()

    def finish(self):
        if self.run["actor"] is not None:
            try:
                self.poll_combat()
                self.record_credit()
            except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
                self.run["capture_complete"] = False
                self.run["terminal_capture_error"] = str(error)
        self.run["end_ms"] = int(time.time() * 1000)
        self.run["elapsed_ms"] = int((time.monotonic() - self.entered_monotonic) * 1000) if hasattr(self, "entered_monotonic") else 0
        publish(self.output / "run.json", self.run)
        events = parse_combat("\n".join(self.events.elements()))
        (self.output / "combat.log").write_text("\n".join(row["text"] for row in events) + "\n")
        report = summarize_dungeon(self.run, events)
        report["failure"] = self.run["failure"]
        publish(self.output / "report.json", report)
        lines = [f"# Spec{self.args.spec} Stockade", f"Clear: {report['clear_complete']}; clear_ms: {report['clear_time_ms']}",
                 f"Player deaths: {report['player_deaths']}; party deaths: {report['party_deaths']}"]
        lines += [f"{boss['name']}: damage={boss['damage']}; DPS={boss['dps']}; complete={boss['measurement_complete']}" for boss in report["bosses"]]
        (self.output / "summary.md").write_text("\n\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("engine", "server", "world-db", "navmesh", "output", "socket", "server-log"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--character", required=True)
    parser.add_argument("--spec", type=int, default=72)
    parser.add_argument("--exit-only", action="store_true", help="Walk a persisted owner through the authored exit before a fresh run")
    parser.add_argument("--timeout", type=float, default=7200)
    args = parser.parse_args()
    if args.timeout <= 0 or (args.output / "run.json").exists():
        parser.error("Positive time bound and unused output required")
    controller = StockadeRun(args)
    if args.exit_only:
        controller.exit_instance()
    else:
        controller.execute()


if __name__ == "__main__":
    main()
