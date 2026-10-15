"""Observed combat outcomes and sampled timing; never infer spending from net power."""
import json
from collections import Counter, defaultdict

from bot_harness import classify_damage_source, ownership_graph

EFFECT_FIELDS = ("ID", "SpellID", "EffectIndex", "Effect", "EffectAura", "EffectBasePointsF",
                 "EffectBonusCoefficient", "BonusCoefficientFromAP", "Coefficient", "Variance",
                 "ResourceCoefficient", "EffectPointsPerResource", "EffectRealPointsPerLevel",
                 "EffectTriggerSpell", "EffectMiscValue_0", "EffectMiscValue_1",
                 "ImplicitTarget_0", "ImplicitTarget_1")


def spell_key(value):
    return None if value in (None, "null") else int(value)


def source_effects_by_spell(metadata):
    rows = defaultdict(dict)
    for spell in metadata.values():
        source = spell.get("source")
        if source is None:
            continue
        for row in source["SpellEffect"] + source["damage_effects"]:
            rows[int(row["SpellID"])][row["ID"]] = {key: row.get(key) for key in EFFECT_FIELDS}
    return {spell: sorted(effects.values(), key=lambda row: (int(row["EffectIndex"]), int(row["ID"])))
            for spell, effects in rows.items()}


def damage_outcomes(manifest, events):
    owners = ownership_graph(events, manifest.get("owner_snapshot", []))
    party = {str(actor) for actor in manifest.get("party_actors", [])}
    hits, misses, damage, names = defaultdict(list), defaultdict(Counter), Counter(), defaultdict(set)
    casts, swing_actors = Counter(), Counter()
    for event in events:
        if not manifest["start_ms"] <= event["at_ms"] < manifest["end_ms"]:
            continue
        if event["kind"] not in ("cast_success", "damage"):
            continue
        if classify_damage_source(event["src"], event["at_ms"], str(manifest["actor"]), owners, party) != "owned":
            continue
        spell = event["spell"]
        if event["spell_name"]:
            names[spell].add(event["spell_name"])
        if event["kind"] == "cast_success":
            casts[spell] += 1
            continue
        if event["dst"] != str(manifest["target"]):
            continue
        if event["miss_kind"]:
            misses[spell][event["miss_kind"]] += 1
        elif event["amount_observed"]:
            hits[spell].append(event)
            damage[spell] += max(0, event["amount"])
            if spell is None:
                swing_actors[event["src"]] += max(0, event["amount"])
    outcomes = {}
    for spell in set(hits) | set(misses) | set(casts):
        rows = hits[spell]
        known_crits = all(row["crit"] is not None for row in rows)
        outcomes[spell] = {"owned_successes": casts[spell], "hits": len(rows),
                           "crits": sum(row["crit"] is True for row in rows) if known_crits else None,
                           "misses": sum(misses[spell].values()), "miss_by_kind": dict(misses[spell])}
    total = sum(damage.values())
    return {"by_spell": outcomes, "names": names,
            "auto_attack": {"damage": damage[None], "share": damage[None] / total if total else None,
                            "by_actor": dict(swing_actors)}}


def sampled_usage(manifest, samples):
    covered = gcd_busy = cast_or_gcd_busy = maximum_gap = 0
    for left, right in zip(samples, samples[1:]):
        if left["at_ms"] > right["at_ms"]:
            raise ValueError("Native sample timestamps are out of order")
        start = max(manifest["start_ms"], left["at_ms"])
        end = min(manifest["end_ms"], right["at_ms"])
        if end <= start:
            continue
        interval = end - start
        remaining = max(0, left["spells"]["gcd_ms"] - (start - left["at_ms"]))
        gcd = min(interval, remaining)
        covered += interval
        gcd_busy += gcd
        cast_or_gcd_busy += interval if left["spells"]["casting"] else gcd
        maximum_gap = max(maximum_gap, interval)
    return {"covered_ms": covered, "maximum_gap_ms": maximum_gap,
            "unobserved_edge_ms": manifest["end_ms"] - manifest["start_ms"] - covered,
            "sampled_gcd_busy_ms": gcd_busy, "sampled_cast_or_gcd_busy_ms": cast_or_gcd_busy,
            "sampled_idle_ms": covered - cast_or_gcd_busy,
            "sampled_gcd_utilization": gcd_busy / covered if covered else None,
            "method": "left-sampled intervals between observations, GCD remaining clipped to next sample; not exact server occupancy"}


def resource_net_change(samples):
    if not samples:
        return None
    readings = []
    for state in (samples[0], samples[-1]):
        encoded = state["assisted"].get("evaluation_json")
        if not encoded:
            return None
        facts = json.loads(encoded)["facts"]
        if "powers" not in facts:
            return None
        readings.append(facts["powers"])
    first, last = readings
    powers = {power: {"first": first.get(power), "last": last.get(power),
                      "net": last[power] - first[power] if power in first and power in last else None}
              for power in sorted(set(first) | set(last))}
    return {"from_at_ms": samples[0]["at_ms"], "to_at_ms": samples[-1]["at_ms"], "powers": powers}
