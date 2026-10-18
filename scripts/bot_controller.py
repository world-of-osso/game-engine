"""Native player actions shared by dummy bots and routed dungeon controllers.

Callers supply checked IPC, fresh target-bound native snapshots, a strict baked
route query and a source-backed per-spec priority. No default rotation or route.
This module neither provisions players nor changes gameplay/server state.
"""
import json
import math
import re
import time
import uuid
from collections import Counter
from pathlib import Path


def publish(path, value):
    """Single writer per owned directory; readers never see truncated JSON."""
    path = Path(path)
    temporary = path.with_suffix(".writing")
    temporary.write_text(json.dumps(value) + "\n")
    temporary.replace(path)


class NoHostileTarget(ValueError):
    """One native Tab produced no observed living hostile target."""


class FileBridge:
    """One outstanding input per owned client; timeout never repeats input."""

    def __init__(self, directory, timeout=10):
        self.directory = Path(directory)
        self.timeout = timeout

    def request(self, kind, payload):
        if kind not in ("assist", "loot-click", "pet-assist", "enemy-tab"):
            raise ValueError(f"Unsupported native input request: {kind}")
        identity = uuid.uuid4().hex
        request = self.directory / f"native-{kind}-request.json"
        publish(request, {**payload, "request_id": identity})
        response = self.directory / f"native-{kind}-result.json"
        deadline = time.monotonic() + self.timeout
        try:
            while time.monotonic() < deadline:
                if response.exists():
                    result = json.loads(response.read_text())
                    if result.get("request_id") == identity:
                        if result.get("error"):
                            raise RuntimeError(f"Native {kind}: {result['error']}")
                        return result
                time.sleep(.01)
            raise TimeoutError(f"Native {kind} timed out; input not resubmitted")
        finally:
            request.unlink(missing_ok=True)


def requested_snapshot(state, target):
    """Adapt the dummy observer's atomic sample without changing query facts."""
    query = state["priority"]
    if query.get("error"):
        raise ValueError(query["error"])
    if query.get("target") != int(target):
        raise ValueError("Native priority query belongs to another target")
    return {**query, "spells": state["spells"], "at_ms": state["at_ms"],
            "actor": int(state["account"]["local_player_id"])}


def owner_in_combat(snapshot):
    assisted = snapshot.get("assisted")
    if not isinstance(assisted, dict) or not isinstance(assisted.get("in_combat"), bool):
        raise ValueError("Native owner combat status missing or invalid")
    return assisted["in_combat"]


def opening_abandonment(snapshot, target):
    if owner_in_combat(snapshot):
        return {"abandoned": "received owner CombatStatus true before cast"}
    if snapshot.get("target") != int(target):
        raise ValueError("Native opening target observation changed")
    health = snapshot["spells"].get("target_health")
    if not isinstance(health, (int, float)) or health < 0:
        raise ValueError("Native opening target health unavailable")
    if health == 0:
        return {"abandoned": "received selected target health zero before cast"}
    return None


def native_usable(snapshot, spell):
    if set(snapshot["spells"]["known"]) != set(snapshot["facts"]["known"]):
        raise ValueError("Native learned-spell snapshots disagree")
    if spell not in snapshot["facts"]["known"]:
        raise ValueError(f"Priority selected unlearned spell {spell}")
    usable = True
    for field in ("ready", "affordable", "in_range"):
        value = snapshot["facts"][field][str(spell)]
        if set(value) != {"Ok"} or not isinstance(value["Ok"], bool):
            raise ValueError(f"Native spell {spell} not usable: unavailable {field}={value}")
        usable = usable and value["Ok"]
    return usable


def require_point(point):
    if len(point) != 3 or not all(math.isfinite(v) for v in point):
        raise ValueError(f"Invalid sourced route point: {point}")


class PlayerController:
    def __init__(self, invoke, snapshot, route, bridge):
        self.invoke = invoke  # Checked existing CLI; failures must raise.
        self.snapshot = snapshot  # Fresh native snapshot bound to requested target.
        self.route = route  # (start, goal, label) -> authored/navmesh corners.
        self.bridge = bridge
        self.pending = None
        self.outcomes = []

    @staticmethod
    def success_counts(state):
        return Counter((int(row["source"]), int(row["spell_id"]), int(row["at_ms"]))
                       for row in state["spells"]["cast_success_events"])

    def observe_cast_outcome(self, state):
        """Receipt, not an IPC enqueue response or elapsed GCD, releases input."""
        pending = self.pending
        if pending is None:
            return True
        failures = [row for row in state["spells"]["cast_failure_events"]
                    if row["sequence"] > pending["failure_seq"]
                    and int(row["spell_id"]) == pending["spell"]]
        successes = self.success_counts(state) - pending["successes"]
        matched = [key for key in successes if key[:2] == (pending["actor"], pending["spell"])]
        if failures or matched:
            outcome = {"spell": pending["spell"], "requested_at_ms": pending["at_ms"],
                       "observed_at_ms": state["at_ms"], "success": bool(matched)}
            if failures:
                outcome.update(failures[0])
            self.outcomes.append(outcome)
            self.pending = None
            return True
        if time.monotonic() >= pending["deadline"]:
            raise TimeoutError(f"Native spell {pending['spell']} outcome not observed; request not repeated")
        return False

    def cast_next(self, target, priority):
        state = self.snapshot(target)
        if state.get("target") != int(target):
            raise ValueError("Native priority facts belong to another target")
        if not self.observe_cast_outcome(state):
            return None
        spells = state["spells"]
        if spells["casting"] or spells["gcd_ms"] > 0:
            return None
        selected = priority(state, target)
        if selected is None:
            return None
        spell = int(selected["spell"])
        if not native_usable(state, spell):
            return None
        response = self.submit_cast(state, target, spell)
        return {**selected, "response": response, "native": state}

    def submit_cast(self, state, target, spell):
        self.pending = {"spell": spell, "actor": int(state["actor"]), "at_ms": state["at_ms"],
                        "successes": self.success_counts(state),
                        "failure_seq": state["spells"]["cast_failure_seq"], "deadline": time.monotonic() + 10}
        return self.invoke("spell", "cast", "--spell", str(spell), "--target", str(target))

    def cast_opening(self, target, spell, admission=None):
        """One prepull outcome or uncast abandonment; submitted casts still need ACK."""
        if self.pending is not None:
            raise RuntimeError("Opening cannot replace an outstanding cast")
        state = self.snapshot(target)
        abandoned = opening_abandonment(state, target)
        if abandoned is not None:
            return abandoned
        if admission is not None:
            state = admission(state, target)
            abandoned = opening_abandonment(state, target)
            if abandoned is not None:
                return abandoned
        if spell not in state["spells"]["known"]:
            raise ValueError("Opening requires a learned spell")
        if not native_usable(state, spell):
            raise ValueError("Opening requires fresh native readiness, affordability and range")
        self.submit_cast(state, target, spell)
        while not self.observe_cast_outcome(self.snapshot(target)):
            time.sleep(.05)
        outcome = self.outcomes[-1]
        if not outcome["success"]:
            raise RuntimeError(f"Native opening refused: {outcome}")
        return outcome

    def step_toward(self, here, goal, label):
        """One bounded physical step; caller reobserves combat/route after it."""
        require_point(here)
        require_point(goal)
        if math.dist(here, goal) < .5:
            return True
        corners = list(self.route(here, goal, label))
        if not corners:
            raise ValueError("Sourced route returned no corners")
        for corner in corners:
            require_point(corner)
            dx, dz = corner[0] - here[0], corner[2] - here[2]
            distance = math.hypot(dx, dz)
            if distance < .5:
                continue
            seconds = min(.55, distance / 7)
            yaw = math.degrees(math.atan2(dx, dz))
            self.invoke("movement", "forward", "--seconds", str(seconds), "--yaw-degrees", str(yaw))
            return False
        # Detour returns the walkable surface, not the requested NPC/model height.
        endpoint = corners[-1]
        goal_gap = math.hypot(endpoint[0] - goal[0], endpoint[2] - goal[2])
        if goal_gap < .5 and math.dist(here, endpoint) < .5:
            return True
        raise ValueError("Sourced route exhausted before goal; no direct-line fallback")

    def walk_route(self, goals, position, observe, timeout):
        """Walk authored goals; observe returns True while combat/recovery owns input.

        Service native offers even during busy/recovery ticks. The caller owns
        native party health/credit checks and explicit per-spec combat decisions.
        """
        deadline = time.monotonic() + timeout
        for goal in goals:
            require_point(goal)
            stuck = 0
            while math.dist(position(), goal) >= .5:
                if time.monotonic() >= deadline:
                    raise TimeoutError("Authored route exceeded bounded execution time")
                self.vote_loot()
                if observe():
                    time.sleep(.1)
                    continue
                before = position()
                if self.step_toward(before, goal, "authored-leg"):
                    break
                time.sleep(.7)
                after = position()
                progressed = math.hypot(after[0] - before[0], after[2] - before[2]) >= .15
                stuck = 0 if progressed else stuck + 1
                if stuck >= 8:
                    raise RuntimeError(f"Physical route blocked at {after}; goal {goal}")

    def assist(self, leader_name):
        self.invoke("quest", "interact", "--npc", leader_name)
        result = self.bridge.request("assist", {"leader_name": leader_name})
        before, after = result["before"], result["after"]
        if before.get("target_name") != leader_name:
            raise ValueError("Native assist did not start on requested leader")
        target = after.get("target")
        return target if target is not None and target != before.get("target") else None

    def tab_to_enemy(self):
        """One native Tab input; no raw entity-selection command or alternate key."""
        result = self.bridge.request("enemy-tab", {})
        target = result["after"].get("target")
        if (
            target is None
            or result.get("selected_hostile") is not True
            or result.get("selected_health", -1) <= 0
        ):
            raise NoHostileTarget("Native Tab did not observe an alive hostile target")
        return int(target)

    def vote_loot(self):
        """Click actual visible non-Need choice; retain Greed vs Transmog identity."""
        tree = self.invoke("dump-ui-tree", "--filter", "GroupLoot")
        pattern = (r"^\s*(GroupLootFrame\d+(?:Greed|Transmog)Button) \[Button\].*? visible "
                   r".*? x=([-\d.]+) y=([-\d.]+) w=([\d.]+) h=([\d.]+)")
        match = re.search(pattern, tree, re.MULTILINE)
        if match is None:
            return None
        name = match.group(1)
        x, y, width, height = map(float, match.groups()[1:])
        if width <= 0 or height <= 0:
            raise ValueError("Native loot button has empty semantic bounds")
        return self.bridge.request("loot-click", {"name": name, "x": x + width / 2, "y": y + height / 2})
