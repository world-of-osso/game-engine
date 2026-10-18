"""HARNESS_PULL_COVERAGE_KNOWN_VISIBLE v2 and HARNESS_PULL_CORRIDOR_REFUSAL.

Admission policies, not a server/Retail completeness or future-chase guarantee.
"""
import math

# game-server crates/server/src/interest.rs INTEREST_RADIUS, ordinary same-copy visibility.
ORDINARY_INTEREST_RADIUS = 100.0
OBSERVATION_MAX_AGE_MS = 10000


def require_snapshot(snapshot):
    if not isinstance(snapshot, dict) or snapshot.get('full_update_after_entry') is not True:
        raise ValueError('Pull coverage UNKNOWN: no full replication update since entry/teleport')
    if not isinstance(snapshot.get('units'), list) or not isinstance(snapshot.get('epoch'), list):
        raise ValueError('Pull coverage UNKNOWN: missing received units/entry epoch')
    if not isinstance(snapshot.get('observed_dying_entries'), list):
        raise ValueError('Pull coverage UNKNOWN: missing since-entry death observations')
    identities = [u['id'] for u in snapshot['units']]
    if len(identities) != len(set(identities)):
        raise ValueError('Pull coverage UNKNOWN: duplicate received identities')
    return {u['id']: u for u in snapshot['units']}


def require_point(point):
    if (not isinstance(point, (list, tuple)) or len(point) != 3
            or any(isinstance(v, bool) or not isinstance(v, (int, float))
                   or not math.isfinite(v) for v in point)):
        raise ValueError('Pull corridor requires finite3D positions')


def positive_radius(radius):
    if (isinstance(radius, bool) or not isinstance(radius, (int, float))
            or not math.isfinite(radius) or radius <= 0):
        raise ValueError('Pull coverage UNKNOWN: sourced aggro radius unavailable')
    return radius


def segment_distance_squared(start, end, point):
    direction = [b - a for a, b in zip(start, end)]
    offset = [p - a for a, p in zip(start, point)]
    length = sum(v * v for v in direction)
    fraction = min(1, max(0, sum(a * b for a, b in zip(offset, direction)) / length)) if length else 0
    return sum((p - a - fraction * d) ** 2 for p, a, d in zip(point, start, direction))


def require_safe_corridor(snapshot, tank_id, target_id, corners, sources):
    units = require_snapshot(snapshot)
    if tank_id not in units or target_id not in units:
        raise ValueError('Pull coverage UNKNOWN: tank or selected target not received')
    tank, target = units[tank_id], units[target_id]
    path = [tank['position'], *corners]
    if not corners:
        raise ValueError('Pull corridor requires complete strict nav corners')
    for point in path:
        require_point(point)
    population = sources.population(tank)
    visible = {u['entry'] for u in units.values()}
    accounted = visible | set(snapshot['observed_dying_entries'])
    missing = set(population['hideable_entries']) - accounted
    if missing:
        raise ValueError(f'Pull coverage UNKNOWN: hideable templates unobserved {sorted(missing)}')
    candidates = []
    for unit in units.values():
        if not isinstance(unit['hostile'], bool) or not isinstance(unit['combat'], bool):
            raise ValueError('Pull coverage UNKNOWN: hostility/combat unavailable')
        if not math.isfinite(unit['health']):
            raise ValueError('Pull coverage UNKNOWN: life unavailable')
        if unit['id'] != target_id and unit['hostile'] and unit['health'] > 0 and not unit['combat']:
            require_point(unit['position'])
            candidates.append((unit, positive_radius(sources.radius(unit, tank))))
    radius = max([positive_radius(population['max_radius']), *[r for _, r in candidates]])
    if any(math.dist(point, tank['position']) + radius > ORDINARY_INTEREST_RADIUS for point in path):
        raise ValueError('Pull coverage UNKNOWN: padded corridor outside ordinary100yd interest')
    for unit, radius in candidates:
        if any(segment_distance_squared(a, b, unit['position']) < radius ** 2
               for a, b in zip(path, path[1:])):
            raise ValueError(f"Pull corridor crosses idle hostile {unit['id']} sourced aggro sphere")
    return {'coverage_policy': 'HARNESS_PULL_COVERAGE_KNOWN_VISIBLE v2',
            'pull_policy': 'HARNESS_PULL_CORRIDOR_REFUSAL', 'epoch': snapshot['epoch'],
            'tank': tank_id, 'target': target_id, 'route': path, 'population': population,
            'idle_hostiles': [{'id': u['id'], 'position': u['position'], 'radius': r}
                              for u, r in candidates]}
