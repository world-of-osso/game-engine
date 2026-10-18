"""Authored Stockade fixture data and native state adapters; no private driver copy."""
import json
import math
import re
import sqlite3
from contextlib import closing
from pathlib import Path

from bot_rotation import authored_next


def admitted_copy(server_text, character_id):
    pattern = (rf"Created instance (\d+) of map 34 at difficulty 1 for InstanceEntrant "
               rf"\{{ character_id: {int(character_id)},")
    copies = {int(value) for value in re.findall(pattern, server_text)}
    if len(copies) != 1:
        raise ValueError(f"Native admitted Stockade copy is not unique: {sorted(copies)}")
    return copies.pop()


def parse_npc(text, entry):
    pattern = (r"^(.+) \(entity (\d+)\) level (\d+) health ([\d.]+)/([\d.]+) "
               r"difficulty (\d+) combat=(\{.*\})$")
    matches = list(re.finditer(pattern, text, re.MULTILINE))
    if len(matches) != 1:
        raise ValueError(f"Expected one unique live NPC for entry {entry}, found {len(matches)}")
    row = matches[0]
    return {"name": row[1], "entity": row[2], "entry": entry, "level": int(row[3]),
            "health": float(row[4]), "max_health": float(row[5]), "difficulty": int(row[6]),
            "combat": json.loads(row[7])}


def encounter_states(server_text, instance):
    pattern = (rf"Instance InMap \{{ map_id: 34, instance_id: {int(instance)} \}}: "
               r"boss \d+ \(encounter (\d+)\) \w+ -> (\w+)")
    return {int(encounter): state for encounter, state in re.findall(pattern, server_text)}


def target_observation_ready(state, target):
    if state.get("target") != int(target) or state.get("facts") is None:
        return False
    if state["spells"].get("target_health") == 0:
        return True
    encoded = state["assisted"].get("evaluation_json")
    return bool(encoded) and json.loads(encoded)["target"] == int(target)


def native_visibility(state, target):
    visibility = state.get("visibility", {})
    if visibility.get("error"):
        raise ValueError(f"Native opening visibility unavailable: {visibility['error']}")
    if visibility.get("target") != int(target) or not isinstance(visibility.get("visible"), bool):
        raise ValueError("Native opening visibility missing or belongs to another target")
    return visibility["visible"]


def authored_priority(state, target):
    if state.get("target") == int(target) and state.get("spells", {}).get("target_health") == 0:
        return None
    assisted = state["assisted"]
    spell = authored_next(assisted)
    evaluation = json.loads(assisted["evaluation_json"])
    if evaluation["target"] != int(target) or state["target"] != int(target):
        raise ValueError("Authored selection belongs to another native target")
    return {"spell": spell, "source": "native AssistedCombatStep evaluation"} if spell else None


def parse_party(text):
    lines = text.splitlines()
    if not lines or not lines[0].startswith("group_roster: "):
        raise ValueError("Native group roster header absent")
    expected = int(lines[0].split(": ")[1])
    result = []
    for line in lines[1:]:
        if line == "-":
            continue
        name, metadata = line.split(" leader=", 1)
        fields = dict(token.split("=", 1) for token in ("leader=" + metadata).split() if "=" in token)
        if not {"hp", "death", "pos", "role", "online"}.issubset(fields):
            raise ValueError("Native group live health/position facts absent")
        current, maximum = map(float, fields["hp"].split("/"))
        position = [float(value) for value in fields["pos"].split(",")]
        if len(position) != 3 or not all(math.isfinite(value) for value in position):
            raise ValueError("Native group member position invalid")
        power = None
        if "power" in fields:
            kind, values = fields["power"].split(":")
            power = [kind, *map(float, values.split("/"))]
        result.append({"name": name, "role": fields["role"], "online": fields["online"] == "true",
                       "health": current, "max_health": maximum, "death": fields["death"],
                       "position": position, "power": power})
    if len(result) != expected:
        raise ValueError("Native group roster count disagrees with live members")
    return result


def stockade_exit_plan(database):
    with closing(sqlite3.connect(Path(database).resolve().as_uri() + "?mode=ro", uri=True)) as db:
        rows = db.execute("SELECT a.ID,a.Pos_0,a.Pos_1,a.Pos_2,w.ID,w.MapID,w.LocX,w.LocY,w.LocZ,"
                          "a.ShapeType,a.Box_length,a.Box_width,a.Box_height,a.Box_yaw "
                          "FROM area_trigger a JOIN tdb_areatrigger_teleport t ON t.ID=a.ID "
                          "JOIN tdb_world_safe_locs w ON w.ID=t.PortLocID "
                          "WHERE a.ContinentID=34 AND w.MapID<>34").fetchall()
    if len(rows) != 1:
        raise ValueError("Expected one authored Stockade exit trigger and destination")
    trigger, x, y, z, safe_loc, map_id, dx, dy, dz, shape, length, width, height, yaw = rows[0]
    if shape != 1 or min(length, width, height) <= 0:
        raise ValueError("Authored Stockade exit requires its valid box trigger shape")
    return {"trigger_id": trigger, "goal": [x, z, -y], "destination_id": safe_loc,
            "destination_map": map_id, "destination": [dx, dz, -dy],
            "box": {"length": length, "width": width, "height": height, "yaw": yaw}}


def inside_exit_trigger(point, plan):
    # Same WoW-coordinate box transform as server maps/store.rs AreaTriggerShape::contains.
    dx, dy, dz = (point[0] - plan["goal"][0], -(point[2] - plan["goal"][2]),
                  point[1] - plan["goal"][1])
    box = plan["box"]
    sine, cosine = math.sin(-box["yaw"]), math.cos(-box["yaw"])
    return (abs(dx * cosine - dy * sine) <= box["length"] / 2
            and abs(dy * cosine + dx * sine) <= box["width"] / 2
            and abs(dz) <= box["height"] / 2)


def stockade_plan(database):
    with closing(sqlite3.connect(Path(database).resolve().as_uri() + "?mode=ro", uri=True)) as db:
        entrance = db.execute("SELECT MapID,LocX,LocY,LocZ FROM tdb_world_safe_locs WHERE ID=3599").fetchone()
        rows = db.execute("SELECT e.ID,e.Name_lang,t.entry,c.guid,c.position_x,c.position_y,c.position_z "
                          "FROM dungeon_encounter e JOIN content_creature_template t ON t.name=e.Name_lang "
                          "JOIN content_creature c ON c.id1=t.entry AND c.map=e.MapID "
                          "WHERE e.MapID=34 AND e.DifficultyID=1").fetchall()
    if entrance is None or entrance[0] != 34 or len(rows) != 3 or len({row[0] for row in rows}) != 3:
        raise ValueError("Authored Stockade entrance and three unique boss spawns required")
    origin = [entrance[1], entrance[3], -entrance[2]]
    bosses = [{"encounter": encounter, "name": name, "entry": entry, "spawn_guid": guid,
               "goal": [x, z, -y], "state": "NotStarted"}
              for encounter, name, entry, guid, x, y, z in rows]
    bosses.sort(key=lambda boss: math.dist(origin, boss["goal"]))
    return {"map": 34, "difficulty": 1, "entrance_id": 3599, "entrance": origin,
            "bosses": bosses, "route_policy": "entrance-distance ordered authored boss goals; strict native navmesh corners only"}
