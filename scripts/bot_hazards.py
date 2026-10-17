"""HARNESS_RAIN_OF_FIRE_ESCAPE: harness escape planning, not Retail AI policy.

Only ordinary-client RainOfFire (86636) spheres reported by the server are
considered; shape and radius come from the snapshot, never guessed here.
"""
import math

RAIN_OF_FIRE_SPELL = 86636
# Existing nav arrival contract (XZ yards), not a hazard radius tuning knob.
ARRIVAL_TOLERANCE = 0.5
HEADINGS = 8


class UnreachableRoute(RuntimeError):
    """Caller maps exactly the known native path refusal to this error."""


def _point(value, name):
    if not isinstance(value, (list, tuple)) or len(value) != 3:
        raise ValueError(f"Invalid {name}: expected [x, y, z], got {value!r}")
    if not all(isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v)
               for v in value):
        raise ValueError(f"Invalid {name}: non-finite or non-numeric {value!r}")
    return [float(v) for v in value]


def _positive_int(entry, key):
    value = entry.get(key)
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise ValueError(f"Area {key} must be a positive integer, got {value!r}")
    return value


def _area(entry):
    if not isinstance(entry, dict):
        raise ValueError(f"Area entry must be an object, got {entry!r}")
    spell = entry.get("spell")
    if not isinstance(spell, int) or isinstance(spell, bool):
        raise ValueError(f"Area spell must be an integer, got {spell!r}")
    if spell != RAIN_OF_FIRE_SPELL:
        return None
    hostile = entry.get("hostile")
    if not isinstance(hostile, bool):
        raise ValueError(f"Area hostile must be a boolean, got {hostile!r}")
    if not hostile:
        return None
    radius = entry.get("radius")
    if (not isinstance(radius, (int, float)) or isinstance(radius, bool)
            or not math.isfinite(radius) or radius <= 0):
        raise ValueError(f"Area radius must be positive and finite, got {radius!r}")
    return {"id": _positive_int(entry, "id"), "caster": _positive_int(entry, "caster"),
            "spell": spell, "center": _point(entry.get("center"), "area center"),
            "radius": float(radius), "hostile": True}


def read_areas(snapshot):
    """Hostile RainOfFire spheres; missing or malformed facts raise ValueError."""
    if not isinstance(snapshot, dict) or not isinstance(snapshot.get("areas"), list):
        raise ValueError("Snapshot must carry an explicit 'areas' list")
    return [area for area in map(_area, snapshot["areas"]) if area is not None]


def _sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def _dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _distance_sq(point, center):
    offset = _sub(point, center)
    return _dot(offset, offset)


def _segment_distance_sq(start, end, center):
    direction = _sub(end, start)
    length_sq = _dot(direction, direction)
    t = 0.0 if length_sq == 0 else min(1.0, max(0.0, -_dot(_sub(start, center), direction)
                                                 / length_sq))
    closest = [start[i] + t * direction[i] for i in range(3)]
    return _distance_sq(closest, center)


def _contains(area, point):
    return _distance_sq(point, area["center"]) <= area["radius"] ** 2


def _sphere_is_safe(path, area):
    """Distance from an initially containing centre is convex per segment, so it
    never decreases on a segment iff it does not decrease at the segment start."""
    center, radius_sq = area["center"], area["radius"] ** 2
    inside = _contains(area, path[0])
    for start, end in zip(path, path[1:]):
        if inside:
            if _dot(_sub(start, center), _sub(end, start)) < 0:
                return False
            inside = _contains(area, end)
        elif _segment_distance_sq(start, end, center) <= radius_sq:
            return False
    return not inside


def route_is_safe(here, corners, areas):
    """Every full 3D segment of here→corners against all inclusive spheres."""
    if not isinstance(corners, (list, tuple)) or not corners:
        raise ValueError(f"Route must have at least one corner, got {corners!r}")
    path = [_point(here, "start point")] + [_point(c, "route corner") for c in corners]
    return all(_sphere_is_safe(path, area) for area in areas)


def _path_length(path):
    return sum(math.dist(a, b) for a, b in zip(path, path[1:]))


def _candidate_goals(here, area):
    center, reach = area["center"], area["radius"] + ARRIVAL_TOLERANCE
    ray_x, ray_z = here[0] - center[0], here[2] - center[2]
    base = math.atan2(ray_z, ray_x) if (ray_x or ray_z) else 0.0
    for k in range(HEADINGS):
        angle = base + k * 2 * math.pi / HEADINGS
        yield k * 360 // HEADINGS, [center[0] + reach * math.cos(angle), here[1],
                                    center[2] + reach * math.sin(angle)]


def escape_goal(here, areas, query_route):
    """Nominal goal of the shortest safe actual escape route, or None if clear."""
    here = _point(here, "player position")
    containing = [area for area in areas if _contains(area, here)]
    if not containing:
        return None
    deepest = min(containing, key=lambda a: (
        -(a["radius"] - math.dist(here, a["center"])), a["id"]))
    best, rejected = None, []
    for degrees, goal in _candidate_goals(here, deepest):
        label = f"rain-of-fire-escape-{deepest['id']}-{degrees}"
        try:
            corners = query_route(here, goal, label)
        except UnreachableRoute as error:
            rejected.append(f"{degrees}: unreachable ({error})")
            continue
        if not route_is_safe(here, corners, areas):
            rejected.append(f"{degrees}: crosses or reenters a hazard")
            continue
        end = _point(corners[-1], "route corner")
        if math.hypot(end[0] - goal[0], end[2] - goal[2]) > ARRIVAL_TOLERANCE:
            rejected.append(f"{degrees}: route ends {end}, not at goal {goal}")
            continue
        length = _path_length([here, *map(list, corners)])
        if best is None or length < best[0]:
            best = (length, goal)
    if best is None:
        raise ValueError(f"No safe RainOfFire escape route from {here} out of area "
                         f"{deepest['id']}: {'; '.join(rejected)}")
    return best[1]
