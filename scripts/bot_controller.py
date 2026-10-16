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
from pathlib import Path


def publish(path, value):
    """Single writer per owned directory; readers never see truncated JSON."""
    path = Path(path)
    temporary = path.with_suffix(".writing")
    temporary.write_text(json.dumps(value) + "\n")
    temporary.replace(path)


class FileBridge:
    """One outstanding input per owned client; timeout never repeats input."""

    def __init__(self, directory, timeout=10):
        self.directory = Path(directory)
        self.timeout = timeout

    def request(self, kind, payload):
        if kind not in ("assist", "loot-click"):
            raise ValueError(f"Unsupported native input request: {kind}")
        identity = uuid.uuid4().hex
        publish(self.directory / f"native-{kind}-request.json",
                {**payload, "request_id": identity})
        response = self.directory / f"native-{kind}-result.json"
        deadline = time.monotonic() + self.timeout
        while time.monotonic() < deadline:
            if response.exists():
                result = json.loads(response.read_text())
                if result.get("request_id") == identity:
                    if result.get("error"):
                        raise RuntimeError(f"Native {kind}: {result['error']}")
                    return result
            time.sleep(.01)
        raise TimeoutError(f"Native {kind} timed out; input not resubmitted")


def require_usable(snapshot, spell):
    if set(snapshot["spells"]["known"]) != set(snapshot["facts"]["known"]):
        raise ValueError("Native learned-spell snapshots disagree")
    if spell not in snapshot["facts"]["known"]:
        raise ValueError(f"Priority selected unlearned spell {spell}")
    for field in ("ready", "affordable", "in_range"):
        value = snapshot["facts"][field][str(spell)]
        if value != {"Ok": True}:
            raise ValueError(f"Native spell {spell} not usable: {field}={value}")


def require_point(point):
    if len(point) != 3 or not all(math.isfinite(v) for v in point):
        raise ValueError(f"Invalid sourced route point: {point}")


class PlayerController:
    def __init__(self, invoke, snapshot, route, bridge):
        self.invoke = invoke  # Checked existing CLI; failures must raise.
        self.snapshot = snapshot  # Fresh native snapshot bound to requested target.
        self.route = route  # (start, goal, label) -> authored/navmesh corners.
        self.bridge = bridge

    def cast_next(self, target, priority):
        state = self.snapshot(target)
        if state.get("target") != int(target):
            raise ValueError("Native priority facts belong to another target")
        spells = state["spells"]
        if spells["casting"] or spells["gcd_ms"] > 0:
            return None
        selected = priority(state, target)
        if selected is None:
            return None
        spell = int(selected["spell"])
        require_usable(state, spell)
        response = self.invoke("spell", "cast", "--spell", str(spell), "--target", str(target))
        return {**selected, "response": response, "native": state}

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
        raise ValueError("Sourced route exhausted before goal; no direct-line fallback")

    def assist(self, leader_name):
        self.invoke("quest", "interact", "--npc", leader_name)
        result = self.bridge.request("assist", {"leader_name": leader_name})
        before, after = result["before"], result["after"]
        if before.get("target_name") != leader_name:
            raise ValueError("Native assist did not start on requested leader")
        target = after.get("target")
        return target if target is not None and target != before.get("target") else None

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
