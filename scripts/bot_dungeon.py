"""Native boss accounting; authoritative completion and observed DPS are separate."""
from bot_harness import analyze


def boss_combat(event, identity, party):
    if event["kind"] == "damage":
        return event["dst"] == identity or (event["src"] == identity and event["dst"] in party)
    return event["kind"] == "cast_start" and event["src"] == identity and event["dst"] in party


def summarize_boss(run, boss, events):
    identity = str(boss["entity"])
    party = {str(actor) for actor in [run["actor"], *run["party_actors"]]}
    observed = [row for row in events if run["start_ms"] <= row["at_ms"] < run["end_ms"]]
    combat = [row for row in observed if boss_combat(row, identity, party)]
    deaths = [row for row in observed if row["kind"] == "death" and row["dst"] == identity]
    identity_fields = {key: boss[key] for key in ("entry", "entity", "encounter", "name", "state")}
    if not combat:
        return {**identity_fields, "measurement_complete": False, "damage": 0, "dps": None,
                "duration_ms": None, "damage_by_actor": {}, "unobserved": ["boss_combat_start"]}
    start = combat[0]["at_ms"]
    end = deaths[0]["at_ms"] if len(deaths) == 1 else run["end_ms"]
    if end <= start:
        return {**identity_fields, "measurement_complete": False, "damage": 0, "dps": None,
                "duration_ms": None, "damage_by_actor": {}, "unobserved": ["positive_boss_combat_duration"]}
    complete = run["capture_complete"] and boss["state"] == "Done" and len(deaths) == 1
    # Include the death's final damage bucket; elapsed duration stays end-start.
    manifest = {**run, "scenario": "stockade_boss", "target": identity, "attempts": [], "spells": [],
                "start_ms": start, "end_ms": end + 1, "measurement_complete": complete}
    result = analyze(manifest, events)
    result["duration_ms"] = end - start
    result["dps"] = result["damage"] / ((end - start) / 1000) if result["measurement_complete"] else None
    result.update(identity_fields, combat_start_ms=start, combat_end_ms=end,
                  combat_end_inclusive=True, duration_basis="first observed boss-related combat to native death timestamp")
    if len(deaths) != 1:
        result["unobserved"].append("unique_native_boss_death")
    return result


def unavailable_dungeon(run):
    bosses = [{"entry": boss["entry"], "name": boss["name"], "encounter": boss["encounter"],
               "entity": None, "state": "Unobserved", "measurement_complete": False,
               "damage": None, "duration_ms": None, "dps": None}
              for boss in run["bosses"]]
    return {"schema": 1, "scenario": "stockade", "spec": run["spec"], "actor": None,
            "clear_complete": False, "clear_time_ms": None, "elapsed_ms": run["elapsed_ms"],
            "player_deaths": None, "party_deaths": None, "metrics_complete": False,
            "capture_complete": False, "bosses": bosses, "unobserved": ["native_owner_preflight"]}


def summarize_dungeon(run, events):
    if not run["bosses"]:
        raise ValueError("Dungeon requires an explicit expected boss roster")
    if run["actor"] is None:
        return unavailable_dungeon(run)
    if run["end_ms"] <= run["start_ms"]:
        raise ValueError("Dungeon observation window must have positive duration")
    events = sorted(events, key=lambda row: row["at_ms"])
    window = [row for row in events if run["start_ms"] <= row["at_ms"] < run["end_ms"]]
    bosses = [summarize_boss(run, boss, events) for boss in run["bosses"]]
    party = {str(actor) for actor in [run["actor"], *run["party_actors"]]}
    deaths = [row for row in window if row["kind"] == "death" and row["dst"] in party]
    credited = all(boss["state"] == "Done" for boss in bosses)
    player_deaths = sum(row["dst"] == str(run["actor"]) for row in deaths)
    return {"schema": 1, "scenario": "stockade", "spec": run["spec"], "actor": run["actor"],
            "clear_complete": credited, "clear_time_ms": run["clear_elapsed_ms"] if credited else None,
            "clear_time_basis": "local monotonic elapsed from ordinary entry confirmation to final authoritative Done observation",
            "elapsed_ms": run["elapsed_ms"], "bosses": bosses,
            "player_deaths": player_deaths if run["capture_complete"] else None,
            "party_deaths": len(deaths) if run["capture_complete"] else None,
            "observed_player_deaths": player_deaths, "observed_party_deaths": len(deaths),
            "death_events": deaths, "capture_complete": run["capture_complete"],
            "metrics_complete": run["capture_complete"] and all(boss["measurement_complete"] for boss in bosses)}
