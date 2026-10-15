"""Living-player qualification for live and retained native measurements."""
import json


def require_living_player(account):
    health = account.get("local_player_health")
    if health is None:
        raise RuntimeError("Native player health was not observed")
    if health <= 0:
        raise RuntimeError("Native player is dead (observed health <= 0); no live DPS measurement")


def read_health_states(directory):
    yield json.loads((directory / "native-before.json").read_text())
    with (directory / "native-samples.jsonl").open() as source:
        for line in source:
            yield json.loads(line)


def read_health_qualification(directory):
    values = []
    try:
        for state in read_health_states(directory):
            require_living_player(state["account"])
            values.append(state["account"]["local_player_health"])
        if len(values) < 2:
            raise RuntimeError("No in-window native health samples were observed")
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        return {"valid": False, "error": f"{directory}: {error}"}
    return {"valid": True, "observations": len(values),
            "minimum_health": min(values), "maximum_health": max(values)}


def qualify_report_health(directory, report):
    """Return qualified diagnostics without modifying the original report or files."""
    result = dict(report)
    result["health_qualification"] = read_health_qualification(directory)
    if not result["health_qualification"]["valid"]:
        result["reported_measurement_complete"] = report["measurement_complete"]
        result["reported_dps"] = report["dps"]
        result["measurement_complete"] = False
        result["dps"] = None
    return result
